use std::sync::LazyLock;

use linkify::{LinkFinder, LinkKind};
use simdutf8::basic::{self, Utf8Error};
use thiserror::Error;

use crate::{
    markup::{
        config::{DynConf, StaticConf},
        lex::{CheckboxType, InlineFormat, ListItemKind, Numbering, Token, TokenSpan},
    },
    object::{Object, ObjectSyntax},
    prelude::*,
    tape::Tape,
};

static LINK_FINDER: LazyLock<LinkFinder> = LazyLock::new(|| {
    let mut value = LinkFinder::new();
    value
        .kinds(&[LinkKind::Email, LinkKind::Url])
        .email_domain_must_have_dot(true)
        .url_can_be_iri(true)
        .url_must_have_scheme(false);
    value
});

const PRE_ICANN_TLD: [&[u8]; 7] = [b"com", b"org", b"net", b"int", b"edu", b"gov", b"mil"];

#[derive(Error, Debug)]
pub enum LexerError {
    #[error("Invalid UTF-8")]
    InvalidUtf8(#[from] Utf8Error),
}

#[derive(Debug)]
pub struct MarkupSyntax<'a> {
    /// The input text.
    pub input: &'a [u8],

    /// Dynamic configuration.
    pub dyn_conf: &'a DynConf,

    /// Static configuration.
    pub static_conf: &'a StaticConf,
}

impl<'a> Compile for MarkupSyntax<'a> {
    type Output = Result<Vec<TokenSpan<'a>>, LexerError>;

    fn compile(self) -> Self::Output {
        if !self.static_conf.trusted_mode {
            let this = &self;
            basic::from_utf8(this.input)?;
        }
        let tokens = self.parse_virtual_tokens();
        let mut tokens = self.parse_text_tokens(tokens);
        self.convert_bad_tokens(&mut tokens);
        tokens.pop(); // remove `Eof`
        Ok(tokens)
    }
}

impl<'a> MarkupSyntax<'a> {
    pub const fn new(dyn_conf: &'a DynConf, static_conf: &'a StaticConf, input: &'a [u8]) -> Self {
        Self {
            input,
            dyn_conf,
            static_conf,
        }
    }

    #[must_use]
    #[inline(always)]
    fn default_pgraph_spacing(&self) -> u8 {
        if self.static_conf.single_line_mode {
            1
        } else {
            2
        }
    }

    #[must_use]
    fn parse_virtual_tokens(&self) -> Vec<TokenSpan<'a>> {
        let mut scan = Scanner {
            in_alt_text: false,
            pgraph_spacing: self.default_pgraph_spacing(),
            tokens: vec![],
            open_quotes: Vec::with_capacity(2),
            open_fmts: vec![],
            data_values: vec![],
            not_a_url: vec![],
        };
        let mut tape = Tape::new(self.input);

        // Because these symbols may show up in prose,
        // we should expect them to most likely be plain text first.
        //
        // This means we should minimize the # of match arms.
        while let Some(&ch) = self.input.get(tape.pos) {
            let jump: Option<Tape<'a, u8>> = match ch {
                // ordered by expected frequency
                b'\n' => {
                    scan.pgraph_spacing = self.default_pgraph_spacing();
                    scan.emit_inplace(tape, Token::Newline, 1);
                    // Returning a positive result even though the cursor hasn't moved
                    // results in a negligible performance hit
                    // from copying the tape data structure.
                    // It's more important to maintain semantics.
                    Some(tape)
                }
                b'`' => scan.handle_btick(tape),
                b'$' => scan.handle_dollar(tape, self.static_conf.finance_mode),
                b'-' => scan.handle_dash(tape),
                b'.' => scan.handle_dot(tape, self.static_conf.infer_links),
                b'*' => scan.handle_star(tape),
                b'_' => scan.handle_fmt(tape, InlineFormat::UNDERLINE),
                b'|' => scan.handle_fmt(tape, InlineFormat::HIGHLIGHT),
                b'~' => scan.handle_fmt(tape, InlineFormat::STRIKETHROUGH),
                b'[' => scan.handle_obrac(tape),
                b']' => scan.handle_cbrac(tape),
                b'=' => scan.handle_equals(tape),
                b'"' | b'\'' => scan.handle_quote(tape, tape[tape.pos]),
                b'{' => scan.handle_curly(tape),
                b'\\' => scan.handle_bslash(tape),
                b';' => {
                    // divider comment ';;' handled by editor
                    tape.seek_ch(b'\n');
                    Some(tape)
                }
                _ => None, // includes spaces, tabs
            };
            if let Some(jump) = jump {
                tape = jump;
            }
            tape.adv();
        }
        scan.tokens
            .sort_unstable_by(|t1, t2| t1.start.cmp(&t2.start));
        scan.tokens
            .push(TokenSpan::new(Token::Eof, tape.len(), tape.len()));
        scan.tokens
    }

    #[must_use]
    fn parse_text_tokens(&self, tokens: Vec<TokenSpan<'a>>) -> Vec<TokenSpan<'a>> {
        let mut read = 0;
        let mut text_start = 0;
        let mut pos = 0;
        let mut result = vec![];
        while read < tokens.len() {
            // collect plaintext tokens
            let next = &tokens[read];
            if next.start == pos {
                if pos - text_start != 0 {
                    result.push(TokenSpan::new(Token::Plaintext, text_start, pos));
                }
                result.push(*next);
                read += 1;
                pos += next.len();
                text_start = pos;
            } else {
                pos += 1;
            }
        }
        result
    }

    /// Transforms malformed structures into plaintext, including:
    /// - Links/Embeds without a body
    /// - Empty headings
    /// - Empty list items
    /// - Empty quotes
    /// - Empty math blocks
    /// - Empty code blocks
    ///
    /// Malformed tokens found are marked as plaintext.
    ///
    /// Since macro expansion is handled outside of the compiler, we assume that all macro
    /// invocations produce text at this stage.
    fn convert_bad_tokens(&self, tokens: &mut Vec<TokenSpan<'a>>) {
        use Token::*;
        for i in 0..tokens.len() {
            match tokens[i].token {
                // access by index to satisfy borrow checker
                HeadingMarker { .. } | LineQuoteMarker | ListItemMarker { .. }
                    if !tokens.get(i + 1).is_some_and(|t| t.token.is_content()) =>
                {
                    tokens[i].bind_plain();
                }
                LinkMarker | EmbedMarker
                    if tokens
                        .iter()
                        .find(|t| matches!(t.token, LinkBody { .. }) || t.token.is_content())
                        .is_some_and(|t| matches!(t.token, LinkBody { .. })) =>
                {
                    tokens[i].bind_plain();
                }
                CodeBlock { body, .. } | MathBlock { body } if body.is_empty() => {
                    tokens[i].bind_plain();
                }
                BlockQuoteOpen
                    if tokens
                        .iter()
                        .find(|t| t.token == BlockQuoteClose || t.token.is_content())
                        .is_some_and(|t| t.token.is_content()) =>
                {
                    tokens[i].bind_plain();
                }
                _ => {}
            }
        }
    }
}

/// Encapsulates mutable state shared between different handlers during Pass 1.
/// Invalid UTF-8 substrings are treated as plaintext.
///
/// # Implementation
/// All `handle_X` functions assume cursor is at a valid characters.
/// Matching logic should be optimized by performing the cheapest validation first.
struct Scanner<'a> {
    /// Virtual (non-plaintext) tokens.
    tokens: Vec<TokenSpan<'a>>,

    /// The number of spaces used to distinguish between two different paragraphs.
    ///
    /// This is 1 between single-line components (such as headings) and any other type of component,
    /// and 2 for all other components.
    pgraph_spacing: u8,

    /// True if currently within alt text (validated '[').
    in_alt_text: bool,

    /// A FIFO stack of positions of the first character of openers that
    /// have been resolved but not yet paired with a closer.
    ///
    /// The first element of each pair is the flank type mask.
    open_fmts: Vec<(InlineFormat, usize)>,

    /// A FIFO stack of positions of the first character of block quote openers that
    /// have been resolved but not yet paired with a closer.
    ///
    /// Block quotes can be nested, but the characters used must match.
    ///
    /// The first element of each pair is whether double quotes were used.
    open_quotes: Vec<(bool, usize)>,

    /// Tracks dot (`.`) characters that have already been designated as not being where a
    /// URL was found.
    not_a_url: Vec<usize>,

    data_values: Vec<Object>,
}

impl<'a> Scanner<'a> {
    /// Pushes the token nside the input between the start and end indices.
    /// The end index is exclusive.   
    #[inline]
    fn emit(&mut self, token: Token<'a>, start: usize, end: usize) {
        self.tokens.push(TokenSpan::new(token, start, end));
    }

    /// Pushes the token whose first character is at the current position
    /// and has the given length.
    //do not return tape for convenience, as `pos` might need to be adjusted before exiting handler.
    #[inline]
    fn emit_inplace(&mut self, tape: Tape<'a, u8>, token: Token<'a>, len: usize) {
        self.tokens
            .push(TokenSpan::new(token, tape.pos, tape.pos + len));
    }

    fn handle_curly(&mut self, mut tape: Tape<'a, u8>) {}

    /// Attempts to emit a token if the character cluster
    /// belongs to a flanking token, such as an inline format or link.
    ///
    /// The current position should be the first character in the cluster.
    /// Returns `None` if a token was not emitted.
    ///
    /// If `None` is not returned, the length of `self.unclosed_pairs` is always modified
    /// and the cursor of the returned tape is left at the final character of the cluster.
    #[must_use]
    fn handle_fmt(&mut self, mut tape: Tape<'a, u8>, fmt: InlineFormat) -> Option<Tape<'a, u8>> {
        let start = tape.pos;
        let len = fmt.len();
        if tape.is_l_clear(start) && !tape.is_r_clear(tape.pos) {
            // open
            // lack of lookahead prevents bottleneck
            self.open_fmts.push((fmt, start));
            tape.pos += len - 1;
            return Some(tape);
        } else if tape.is_r_clear(start)
            && self
                .open_fmts
                .last()
                .is_some_and(|(last, _)| last.intersects(fmt))
        {
            // close
            let (open_mask, open_pos) = self.open_fmts.pop().unwrap();
            let open_len = InlineFormat::len(open_mask);
            // unsorted tokens don't matter since tokens are sorted after Pass 1
            if (fmt.bits() & open_mask.bits()).ilog2() == 1 {
                // basic pair
                self.emit(
                    Token::InlineFormat {
                        ty: open_mask,
                        twin_pos: start,
                    },
                    open_pos,
                    open_pos + len,
                );
                self.emit_inplace(
                    tape,
                    Token::InlineFormat {
                        ty: open_mask,
                        twin_pos: open_pos,
                    },
                    open_len,
                );
                tape.pos += open_len;
            } else if fmt == InlineFormat::BOLD_ITALIC && open_mask == InlineFormat::BOLD_ITALIC {
                // stop at next format marker appended to this cluster
                self.emit(
                    Token::InlineFormat {
                        ty: InlineFormat::BOLD,
                        twin_pos: start + 1,
                    },
                    open_pos,
                    open_pos + 2,
                );
                self.emit(
                    Token::InlineFormat {
                        ty: InlineFormat::ITALIC,
                        twin_pos: start,
                    },
                    open_pos + 2,
                    open_pos + 3,
                );
                self.emit_inplace(
                    tape,
                    Token::InlineFormat {
                        ty: InlineFormat::ITALIC,
                        twin_pos: open_pos + 2,
                    },
                    1,
                );
                self.emit(
                    Token::InlineFormat {
                        ty: InlineFormat::BOLD,
                        twin_pos: open_pos,
                    },
                    start + 1,
                    start + 3,
                );
            } else {
                // open_mask == InlineFormat::BOLD_ITALIC
                if fmt == InlineFormat::BOLD {
                    self.open_fmts.push((InlineFormat::ITALIC, open_pos));
                    self.emit(
                        Token::InlineFormat {
                            ty: InlineFormat::BOLD,
                            twin_pos: start,
                        },
                        open_pos + 1,
                        open_pos + 3,
                    );
                    self.emit_inplace(
                        tape,
                        Token::InlineFormat {
                            ty: InlineFormat::BOLD,
                            twin_pos: open_pos + 1,
                        },
                        2,
                    );
                } else {
                    self.open_fmts.push((InlineFormat::BOLD, open_pos));
                    self.emit(
                        Token::InlineFormat {
                            ty: InlineFormat::ITALIC,
                            twin_pos: start,
                        },
                        open_pos + 2,
                        open_pos + 3,
                    );
                    self.emit_inplace(
                        tape,
                        Token::InlineFormat {
                            ty: InlineFormat::ITALIC,
                            twin_pos: open_pos + 2,
                        },
                        1,
                    );
                }
            }
            return Some(tape);
        }
        None
    }

    /// Resolves whether a `'` or `"` character belongs to an admonition, a quote
    /// (shorthand or long-form) or plain text.
    ///
    /// Quote blocks of a different sigil can be nested once.
    /// Unlike fenced code blocks, the quote block handler does not consume
    /// inner content indiscrimantly. Instead, it behaves like a link,
    /// with inner markup being seperate from the token itself.
    #[must_use]
    fn handle_quote(&mut self, mut tape: Tape<'a, u8>, quote: u8) -> Option<Tape<'a, u8>> {
        if !tape.is_cur_prefix() {
            return None;
        }
        let start = tape.pos;
        if tape.is_at(&[quote; 2]) {
            // single-line shorthand
            self.emit_inplace(tape, Token::LineQuoteMarker, 2);
            self.pgraph_spacing = 1;
            tape.pos += 2; // skip over `""`/`''`
            return Some(tape);
        }
        let delim = &[quote; 3];
        if tape.is_at(delim) {
            tape.pos += 3; // skip over `"""`/`'''`
            if let Some(&(double, open_pos)) = self.open_quotes.last()
                && double == (quote == b'"')
            {
                self.emit(Token::BlockQuoteOpen, open_pos, open_pos + 3);
                self.emit_inplace(tape, Token::BlockQuoteClose, 3);
                self.open_quotes.pop();
                return Some(tape);
            }
            self.open_quotes.push((quote == b'"', start));
            return Some(tape);
        }
        None
    }

    /// Resolves whether a '[' character belongs to a link, an embed, an assignment, or plain text.
    #[must_use]
    fn handle_obrac(&mut self, mut tape: Tape<'a, u8>) -> Option<Tape<'a, u8>> {
        if self.in_alt_text {
            return None;
        }
        if let Some(tape) = self.try_assignment(tape) {
            return Some(tape);
        }
        tape.adv(); // skip '['
        tape.poll_in_pgraph(self.pgraph_spacing, |ch, pos| {
            let next = tape[pos + 1];
            ch == b']' && (next == b'(' || next == b'[')
        })?;
        if tape.peek_back() == Some(b'!') {
            self.emit(Token::EmbedMarker, tape.pos - 1, tape.pos + 1);
        } else {
            self.emit_inplace(tape, Token::LinkMarker, 1);
        }
        self.in_alt_text = true;
        Some(tape)
    }

    #[must_use]
    fn try_assignment(&mut self, mut tape: Tape<'a, u8>) -> Option<Tape<'a, u8>> {
        let start = tape.pos;
        tape.adv(); // skip `[`
        tape.consume(|ch, _| ch.is_file_ws());
        tape.next().filter(|ch| ch.is_file_key_start())?;
        let key_start = tape.pos;
        tape.consume(|ch, _| ch.is_file_key_part());
        let key = &tape[key_start..tape.pos];
        tape.consume(|ch, _| ch.is_file_ws());
        tape.next().filter(|&ch| ch == b']')?;
        tape.consume(|ch, _| ch.is_file_ws());
        tape.next().filter(|&ch| ch == b'=')?;
        tape.consume(|ch, _| ch.is_file_ws());
        let (value, len) = ObjectSyntax::new(str::from_utf8(tape.rest()).ok()?)
            .compile()
            .ok()?; // todo warn thru lsp
        tape.pos += len;
        self.emit(
            Token::Assignment {
                key,
                value_idx: self.data_values.len(),
            },
            start,
            tape.pos,
        );
        self.data_values.push(value);
        Some(tape) // allow trailing tokens
    }

    /// Resolves whether a ']' character belongs to a link body, an embed body, or plain text.
    #[must_use]
    fn handle_cbrac(&mut self, mut tape: Tape<'a, u8>) -> Option<Tape<'a, u8>> {
        if !self.in_alt_text {
            return None;
        }
        let spacing = self.pgraph_spacing;
        let stop;
        let start = tape.pos;
        tape.adv(); // skip ']'
        match tape.cur() {
            Some(b'[') => stop = b']',
            Some(b'(') => stop = b')',
            _ => {
                return None;
            }
        }
        let body = tape.consume_in_pgraph(spacing, |ch, _| ch != stop);
        if body.is_empty() || tape.cur() != Some(stop) {
            return None;
        }
        if stop == b']' {
            self.emit(Token::LinkAliasBody { alias: body }, start, tape.pos + 1);
        } else {
            self.emit(Token::LinkBody { href: body }, start, tape.pos + 1);
        }
        Some(tape)
    }

    /// Resolves whether a '.' character belongs to an ordered list item,
    /// an inferred link, or plain text.
    ///
    /// Email and SMS links are too vague, so they are not inferred.
    /// All other link types are too niche.
    /// URIs without a scheme must have a suitable TLD (see `PRE_ICANN_TLD`).
    #[must_use]
    fn handle_dot(&mut self, mut tape: Tape<'a, u8>, infer_links: bool) -> Option<Tape<'a, u8>> {
        if tape.is_cur_prefix() {
            self.emit_inplace(
                tape,
                Token::ListItemMarker {
                    indent: tape.count_indent(),
                    kind: ListItemKind::Continuation,
                },
                1,
            );
            self.pgraph_spacing = 1;
            return Some(tape);
        }
        let prev = tape.peek_back();
        if prev.is_none() {
            return None;
        }
        if tape.is_prefix(tape.pos - 1) {
            self.emit(
                Token::ListItemMarker {
                    indent: tape.count_indent(),
                    kind: ListItemKind::Numbered(Numbering::from_marker(prev.unwrap())?),
                },
                tape.pos - 1,
                tape.pos + 1,
            );
            self.pgraph_spacing = 1;
            return Some(tape);
        }
        if !infer_links {
            return None;
        }
        tape.seek_back(|ch, _| ch.is_file_ws());
        tape.adv();
        let start = tape.pos;
        let href = tape.consume(|ch, _| !ch.is_file_ws());
        let link = LINK_FINDER.links(str::from_utf8(href).ok()?).next()?;
        if *link.kind() == LinkKind::Url
            && !link.as_str().contains("//")
            && !PRE_ICANN_TLD.contains(&link.as_str().as_bytes().tld())
        {
            return None;
        }
        self.emit(Token::InferredLink { href }, start, tape.pos);
        Some(tape)
    }

    /// Resolves whether a '-' character belongs to an unordered list item,
    /// a checkbox, a horizontal rule, or plain text.
    #[must_use]
    fn handle_dash(&mut self, mut tape: Tape<'a, u8>) -> Option<Tape<'a, u8>> {
        if !tape.is_cur_prefix() {
            return None;
        }
        if matches!(tape.peek(), Some(b'o') | Some(b'x') | Some(b'?')) {
            // checkbox
            tape.adv(); // skip '-'
            let marker = tape[tape.pos];
            if marker == b'o' || marker == b'x' {
                tape.peek().filter(|ch| ch.is_file_ws())?;
            }
            self.emit_inplace(
                tape,
                Token::ListItemMarker {
                    indent: tape.count_indent(),
                    kind: ListItemKind::Checkbox(CheckboxType::from_marker(marker)?),
                },
                2,
            );
            tape.adv();
            self.pgraph_spacing = 1;
            return Some(tape); // stop at '-'
        }
        if tape.is_at(b"--") {
            tape.pos += 2 + tape.consume(|ch, _| ch == b'-').len();
            if tape
                .consume(|ch, _| ch != b'\n')
                .iter()
                .all(|ch| ch.is_file_ws())
            {
                self.emit_inplace(tape, Token::HorizontalRule, 3);
                tape.dec();
                return Some(tape); // stop at last '-'
            } else {
                return None;
            }
        }
        self.emit_inplace(
            tape,
            Token::ListItemMarker {
                indent: tape.count_indent(),
                kind: ListItemKind::Unordered,
            },
            1,
        );
        self.pgraph_spacing = 1;
        Some(tape) // stop at '-'
    }

    /// Resolves whether a '=' character belongs to a heading or plain text.
    #[must_use]
    fn handle_equals(&mut self, mut tape: Tape<'a, u8>) -> Option<Tape<'a, u8>> {
        if !tape.is_cur_prefix() {
            return None;
        }
        let start = tape.pos;
        let marker = tape.consume_in_pgraph(1, |ch, _| ch == b'=');
        let depth = marker.len();
        if depth > Token::HEADING_MAX {
            return Some(tape); // treat as text, but skip next few '='
        }
        self.emit(Token::HeadingMarker { depth: depth as u8 }, start, tape.pos);
        self.pgraph_spacing = 1;
        tape.dec();
        Some(tape) // stop at final '='
    }

    /// Resolves whether a '$' character belongs to inline math,
    /// a dollar sign literal (if enabled), or plain text.
    #[must_use]
    fn handle_dollar(
        &mut self,
        mut tape: Tape<'a, u8>,
        finance_mode: bool,
    ) -> Option<Tape<'a, u8>> {
        let start = tape.pos;
        if tape.is_at(b"$$") {
            if !tape.is_cur_prefix() {
                return None;
            }
            tape.pos += 2; // skip over '$$'
            let body_start = tape.pos + 1;
            if !tape.seek_ch3(b'\n', b'$', b'$') {
                // failed lookahead
                return None;
            }

            self.emit(
                Token::MathBlock {
                    body: &tape[body_start..tape.pos],
                },
                start,
                tape.pos + 1,
            );
            tape.pos += 2; // stop at last '$$'
        }
        if finance_mode {
            return None;
        }
        if !tape.seek_ch_in_pgraph(self.pgraph_spacing, b'$') {
            // failed lookahead
            return None; // stop at '$'
        }
        self.tokens.push(TokenSpan::new(
            Token::InlineMath {
                body: &tape[start + 1..tape.pos],
            },
            start,
            tape.pos + 1,
        ));
        Some(tape) // stop at closing '$'
    }

    /// Resolves whether a '`' character belongs to inline code or plain text.
    #[must_use]
    fn handle_btick(&mut self, mut tape: Tape<'a, u8>) -> Option<Tape<'a, u8>> {
        let start = tape.pos;
        let spacing = self.pgraph_spacing;
        if tape.is_at(b"```") {
            if !tape.is_cur_prefix() {
                return None;
            }
            tape.pos += 3; // skip over '```'
            let lang = tape.consume(|ch, _| ch != b'\n');
            let body_start = tape.pos + 1; // after '\n'
            if !tape.seek_at(b"\n```") {
                // failed lookahead
                return None;
            }
            self.emit(
                Token::CodeBlock {
                    body: &tape[body_start..tape.pos],
                    lang: lang.trim_file_ws(),
                },
                start,
                tape.pos + 1,
            );
            tape.pos += 3; // stop at last '`'
            return Some(tape);
        }
        if tape.is_at(b"``") {
            tape.adv(); // skip over first '`' of open
            if !tape.seek_at_in_pgraph(spacing, b"``") {
                return Some(tape); // stop at 2nd '`'; treat as text
            }
            tape.adv(); // skip over first '`' of closer
            self.emit(
                Token::InlineRawCode {
                    body: &tape[start + 2..tape.pos],
                },
                start,
                tape.pos + 1,
            );
            return Some(tape);
        }
        if !tape.seek_ch_in_pgraph(spacing, b'`') {
            // failed lookahead
            return None; // stop at '`'
        }
        self.tokens.push(TokenSpan::new(
            Token::InlineCode {
                body: &tape[start + 1..tape.pos],
            },
            start,
            tape.pos + 1,
        ));
        Some(tape) // stop at closing '`'
    }

    /// Resolves whether a `*` character belongs to a bold token,
    /// an italic token, both, or plain text.
    #[must_use]
    fn handle_star(&mut self, tape: Tape<'a, u8>) -> Option<Tape<'a, u8>> {
        if tape.is_at(b"***") {
            self.handle_fmt(tape, InlineFormat::BOLD | InlineFormat::ITALIC)
        } else if tape.is_at(b"**") {
            self.handle_fmt(tape, InlineFormat::BOLD)
        } else {
            // try for '*'
            self.handle_fmt(tape, InlineFormat::ITALIC)
        }
    }

    /// Resolves whether a `\` character
    /// belongs to an escape character, a macro, or plain text.
    #[must_use]
    fn handle_bslash(&mut self, mut tape: Tape<'a, u8>) -> Option<Tape<'a, u8>> {
        if tape.pos == tape.len() - 1 {
            return None;
        }
        let start = tape.pos; // keep for macro handle token
        tape.adv(); // skip past '\'
        let name = tape.consume_file_key();
        if name.len() == 0 {
            // treat as escape
            return Some(tape); // stop at the character after '\'
        }
        let mut next_pos = tape.pos;
        let mut cur = tape.cur();
        if cur.is_none_or(|ch| ch != b'[' && ch != b'{') {
            // treat as incomplete macro
            return Some(tape); // stop at the first non-WS character after the macro name
        }
        self.tokens.push(TokenSpan::new(
            Token::MacroHandle { name },
            start,
            start + name.len() + 1,
        ));
        if cur == Some(b'(') {
            if !tape.seek_ch(b')') {
                // treat as incomplete macro
                tape.dec();
                return Some(tape); // stop before '('
            }
            tape.adv(); // skip past ')'
            self.emit(
                Token::MacroDeco {
                    body: &tape[next_pos + 1..tape.pos],
                },
                next_pos,
                tape.pos,
            );
            next_pos = tape.pos;
            cur = tape.cur();
            // stop after ')'
        }
        if cur == Some(b'[') {
            if !tape.seek_ch(b']') {
                // treat as incomplete macro
                tape.dec();
                return Some(tape); // stop before '['
            }
            tape.adv(); // skip past ']'
            self.emit(
                Token::MacroConfig {
                    body: &tape[next_pos + 1..tape.pos],
                },
                next_pos,
                tape.pos,
            );
            next_pos = tape.pos;
            cur = tape.cur();
            // stop after ']'
        }
        while cur == Some(b'{') {
            if !tape.seek_ch(b'}') {
                // treat as incomplete macro
                tape.dec();
                return Some(tape); // stop before '{'
            }
            tape.adv(); // skip past '}'
            self.emit(
                Token::MacroBody {
                    body: &tape[next_pos + 1..tape.pos],
                },
                next_pos,
                tape.pos,
            );
            next_pos = tape.pos;
            cur = tape.cur();
            // stop after '}'
        }
        Some(tape)
    }
}
