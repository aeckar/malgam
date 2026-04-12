use simdutf8::basic::{self, Utf8Error};
use thiserror::Error;

use crate::markup::config::{DynConf, StaticConf};
use crate::markup::lexer_utils::{CheckboxType, InlineFormat as fmt, Numbering, Token, TokenSpan};
use crate::prelude::*;
use crate::tape::Tape;

#[derive(Error, Debug)]
pub enum LexerError {
    #[error("Invalid UTF-8")]
    InvalidUtf8(#[from] Utf8Error),
}

/// Encapsulates mutable state shared between different handlers during Pass 1.
struct VirtualLexer<'a> {
    /// Virtual (non-plaintext) tokens.
    tokens: Vec<TokenSpan<'a>>,

    /// The number of spaces used to distinguish between two different paragraphs.
    ///
    /// This is 1 between single-line components (such as headings) and any other type of component,
    /// and 2 for all other components.
    pgraph_spacing: u8,

    /// True if currently within alt text (validated '[').
    in_alt_text: bool,

    /// A stack of positions of the first character of openers that
    /// have been resolved but not yet paired with a closer.
    ///
    /// The first element of each pair is the flank type mask.
    open_fmts: Vec<(u8, usize)>,

    /// A stack of positions of the first character of block quote openers that
    /// have been resolved but not yet paired with a closer.
    ///
    /// Block quotes can be nested, but the characters used must match.
    ///
    /// The first element of each pair is whether double quotes were used.
    open_quotes: Vec<(bool, usize)>,
}

/// All `handle_X` functions assume cursor is at a valid character.
impl<'a> VirtualLexer<'a> {
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

    /// Attempts to emit a token if the character cluster
    /// belongs to a flanking token, such as an inline format or link.
    ///
    /// The current position should be the first character in the cluster.
    /// Returns `None` if a token was not emitted.
    ///
    /// If `None` is not returned, the length of `self.unclosed_pairs` is always modified
    /// and the cursor of the returned tape is left at the final character of the cluster.
    #[must_use]
    fn handle_pair(&mut self, mut tape: Tape<'a, u8>, mask: u8) -> Option<Tape<'a, u8>> {
        const BOLD_ITALIC_MASK: u8 = fmt::BOLD_FLAG | fmt::ITALIC_FLAG;
        const BOLD_TY: Token<'static> = Token::InlineFormat { ty: fmt::Bold };
        const ITALIC_TY: Token<'static> = Token::InlineFormat { ty: fmt::Italic };
        let start = tape.pos;
        let len = fmt::len(mask);
        if tape.is_l_clear(start) && !tape.is_r_clear(tape.pos) {
            // open
            // lack of lookahead prevents bottleneck
            self.open_fmts.push((mask, start));
            tape.pos += len - 1;
            return Some(tape);
        } else if tape.is_r_clear(start)
            && self.open_fmts.last().is_some_and(|(t, _)| *t & mask != 0)
        {
            // close
            let (open_mask, open_pos) = self.open_fmts.pop().unwrap();
            let open_len = fmt::len(open_mask);
            // unsorted tokens don't matter since tokens are sorted after Pass 1
            if (mask & open_mask).ilog2() == 1 {
                // basic pair
                let token = Token::InlineFormat {
                    ty: fmt::from_flag(open_mask),
                };
                self.emit(token, open_pos, open_pos + len);
                self.emit_inplace(tape, token, open_len);
                tape.pos += open_len;
                // if mask == BOLD_ITALIC_MASK: stop at next format marker appended to this cluster
            } else if mask == BOLD_ITALIC_MASK && open_mask == BOLD_ITALIC_MASK {
                self.emit(BOLD_TY, open_pos, open_pos + 2);
                self.emit(ITALIC_TY, open_pos + 2, open_pos + 3);
                self.emit_inplace(tape, ITALIC_TY, 1);
                self.emit(BOLD_TY, start + 1, start + 3);
            } else {
                // open_mask == BOLD_ITALIC_MASK
                if mask == fmt::BOLD_FLAG {
                    self.open_fmts.push((fmt::ITALIC_FLAG, open_pos));
                    self.emit(BOLD_TY, open_pos + 1, open_pos + 3);
                    self.emit_inplace(tape, BOLD_TY, 2);
                } else {
                    self.open_fmts.push((fmt::BOLD_FLAG, open_pos));
                    self.emit(ITALIC_TY, open_pos + 2, open_pos + 3);
                    self.emit_inplace(tape, ITALIC_TY, 1);
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

    /// Resolves whether a '[' character belongs to a link, an embed, or plain text.
    #[must_use]
    fn handle_obrac(&mut self, mut tape: Tape<'a, u8>) -> Option<Tape<'a, u8>> {
        if self.in_alt_text {
            return None;
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

    /// Resolves whether a '.' character belongs to an ordered list item or plain text.
    #[must_use]
    fn handle_dot(&mut self, tape: Tape<'a, u8>) -> Option<Tape<'a, u8>> {
        if tape.is_cur_prefix() {
            self.emit_inplace(
                tape,
                Token::NumberedItemMarker {
                    depth: tape.count_indent(),
                    ty: Numbering::Continuation,
                },
                1,
            );
            self.pgraph_spacing = 1;
            return Some(tape);
        }
        let prev = tape.peek_back();
        if prev.is_none() || !tape.is_prefix(tape.pos - 1) {
            return None;
        }
        self.emit(
            Token::NumberedItemMarker {
                depth: tape.count_indent(),
                ty: Numbering::from_marker(prev.unwrap())?,
            },
            tape.pos - 1,
            tape.pos + 1,
        );
        self.pgraph_spacing = 1;
        Some(tape)
    }

    /// Resolves whether a '-' character belongs to an unordered list item,
    /// a checkbox, a horizontal rule, or plain text.
    #[must_use]
    fn handle_dash(&mut self, mut tape: Tape<'a, u8>) -> Option<Tape<'a, u8>> {
        if matches!(tape.peek_back(), Some(b'o') | Some(b'x') | Some(b'?')) {
            // checkbox
            tape.dec(); // decrement to enable check on line start
            if !tape.is_cur_prefix() {
                return None;
            }
            self.emit_inplace(
                tape,
                Token::Checkbox {
                    depth: tape.count_indent(),
                    ty: CheckboxType::from_marker(tape[tape.pos])?,
                },
                2,
            );
            tape.adv();
            self.pgraph_spacing = 1;
            return Some(tape); // stop at '-'
        }
        if !tape.is_cur_prefix() {
            return None;
        }
        if tape.is_at(b"--") {
            tape.pos += 2;
            let tail = tape.consume(|ch, _| ch != b'\n');
            if tail.iter().all(|ch| ch.is_file_ws()) {
                self.emit_inplace(tape, Token::HorizontalRule, 3);
                tape.dec();
                return Some(tape); // stop at last '-'
            }
        }
        self.emit_inplace(
            tape,
            Token::ListItemMarker {
                depth: tape.count_indent(),
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
        self.emit(Token::Heading { depth: depth as u8 }, start, tape.pos);
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
                    body: &tape.slice(body_start..tape.pos),
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
                body: &tape.slice(start + 1..tape.pos),
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
                    body: &tape.slice(body_start..tape.pos),
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
                    body: &tape.slice(start + 2..tape.pos),
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
                body: &tape.slice(start + 1..tape.pos),
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
            self.handle_pair(tape, fmt::BOLD_FLAG | fmt::ITALIC_FLAG)
        } else if tape.is_at(b"**") {
            self.handle_pair(tape, fmt::BOLD_FLAG)
        } else {
            // try for '*'
            self.handle_pair(tape, fmt::ITALIC_FLAG)
        }
    }

    /// Resolves whether a `\` character
    /// belongs to an escape character, a macro, or plain text.
    #[must_use]
    fn handle_bslash(&mut self, mut tape: Tape<'a, u8>) -> Option<Tape<'a, u8>> {
        if tape.pos == tape.raw.len() - 1 {
            return None;
        }
        let start = tape.pos; // keep for macro handle token
        tape.adv(); // skip past '\'
        let name = tape.consume(|ch, _| ch.is_ascii_alphabetic());
        if name.len() == 0 {
            // treat as escape
            return Some(tape); // stop at the character after '\'
        }
        let mut next_pos = tape.pos;
        let mut next = tape.cur();
        if next.is_none_or(|ch| ch != b'[' && ch != b'{') {
            // treat as incomplete macro
            return Some(tape); // stop at the first non-WS character after the macro name
        }
        self.tokens.push(TokenSpan::new(
            Token::MacroHandle { name },
            start,
            start + name.len() + 1,
        ));
        if next == Some(b'[') {
            if !tape.seek_ch(b']') {
                // treat as incomplete macro
                return Some(tape); // stop at '['
            }
            tape.adv(); // skip past ']'
            self.emit(
                Token::MacroArgs {
                    body: &tape.slice(next_pos + 1..tape.pos),
                },
                next_pos,
                tape.pos,
            );
            next_pos = tape.pos;
            next = tape.cur();
            // stop after ']'
        }
        while next == Some(b'{') {
            if !tape.seek_ch(b'}') {
                // treat as incomplete macro
                return Some(tape); // stop at '{'
            }
            tape.adv(); // skip past '}'
            self.emit(
                Token::MacroBody {
                    body: &tape.slice(next_pos + 1..tape.pos),
                },
                next_pos,
                tape.pos,
            );
            next_pos = tape.pos;
            next = tape.cur();
            // stop after '}'
        }
        Some(tape)
    }
}

/// Draft markup syntax.
#[derive(Debug)]
pub struct Lexer<'a> {
    /// The input text.
    pub input: &'a [u8],

    /// Dynamic configuration.
    pub dyn_conf: &'a DynConf,

    /// Static configuration.
    pub static_conf: &'a StaticConf,
}

impl<'a> Compile for Lexer<'a> {
    type Output = Result<Vec<TokenSpan<'a>>, LexerError>;

    fn compile(self) -> Self::Output {
        if !self.static_conf.trusted_mode {
            self.validate_utf8()?;
        }
        let tokens = self.parse_virtual_tokens();
        let mut tokens = self.parse_text_tokens(tokens);
        self.convert_bad_tokens(&mut tokens);
        tokens.pop(); // remove `Eof`
        Ok(tokens)
    }
}

impl<'a> Lexer<'a> {
    pub fn new(dyn_conf: &'a DynConf, static_conf: &'a StaticConf, input: &'a [u8]) -> Self {
        Self {
            input,
            dyn_conf,
            static_conf,
        }
    }

    #[must_use]
    fn validate_utf8(&self) -> Result<(), LexerError> {
        basic::from_utf8(self.input)?;
        Ok(())
    }

    #[must_use]
    fn parse_virtual_tokens(&self) -> Vec<TokenSpan<'a>> {
        let mut lex = VirtualLexer {
            in_alt_text: false,
            pgraph_spacing: 2,
            tokens: vec![],
            open_quotes: Vec::with_capacity(2),
            open_fmts: vec![],
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
                    lex.pgraph_spacing = 2;
                    lex.emit_inplace(tape, Token::Newline, 1);
                    // Returning a positive result even though the cursor hasn't moved
                    // results in a negligible performance hit
                    // from copying the tape data structure.
                    // It's more important to maintain semantics.
                    Some(tape)
                }
                b'`' => lex.handle_btick(tape),
                b'$' => lex.handle_dollar(tape, self.static_conf.finance_mode),
                b'-' => lex.handle_dash(tape),
                b'.' => lex.handle_dot(tape),
                b'*' => lex.handle_star(tape),
                b'_' => lex.handle_pair(tape, fmt::UNDERLINE_FLAG),
                b'|' => lex.handle_pair(tape, fmt::HIGHLIGHT_FLAG),
                b'~' => lex.handle_pair(tape, fmt::STRIKETHROUGH_FLAG),
                b'[' => lex.handle_obrac(tape),
                b']' => lex.handle_cbrac(tape),
                b'=' => lex.handle_equals(tape),
                b'"' | b'\'' => lex.handle_quote(tape, tape[tape.pos]),
                b'\\' => lex.handle_bslash(tape),
                b';' => {
                    // divider comment ';;' handled by editor
                    tape.seek_ch(b'\n');
                    Some(tape)
                }
                _ => None,
            };
            if let Some(jump) = jump {
                tape = jump;
            }
            tape.adv();
        }
        lex.tokens
            .sort_unstable_by(|t1, t2| t1.start.cmp(&t2.start));
        lex.tokens
            .push(TokenSpan::new(Token::Eof, tape.raw.len(), tape.raw.len()));
        lex.tokens
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
                Heading { .. }
                | LineQuoteMarker
                | ListItemMarker { .. }
                | NumberedItemMarker { .. }
                | Checkbox { .. }
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
