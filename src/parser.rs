//! Don't check for UTF-8 correctness; leave that to the user.

use crate::char_ext::CharExt;
use crate::tape::Tape;
use crate::{Token, TokenType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParserConfig {
    handle_inline_math: bool,
}

/// Contains global parser state.
#[derive(Debug)]
pub struct Parser<'a> {
    /// The number of spaces Use to distinguish between two different paragraphs.
    ///
    /// This is 1 between single-line components (such as headings) and any other type of component,
    /// and 2 for all other components.
    pgraph_spacing: u8,

    /// The positions of all pairs currently resolved.
    ///
    /// The key is the position of the first character in the opener
    /// and the value is one past the position of the last character in the closer.
    pairs: Vec<(usize, usize)>,

    /// A stack of positions of the first character of openers that
    /// have been resolved but not yet paired with a closer.
    unclosed_pairs: Vec<(TokenType, usize)>,

    ///the tokens are generated in the pre-pass, and
    /// then used in the main pass to generate the output.
    tokens: Vec<Token>,

    /// The input text.
    pub input: &'a [u8],

    /// Configuration flags.
    pub config: &'a ParserConfig,
}

impl<'a> Parser<'a> {
    pub fn new(config: &'a ParserConfig, input: &'a [u8]) -> Self {
        Self {
            pgraph_spacing: 2,
            unclosed_pairs: Vec::new(),
            tokens: Vec::new(),
            pairs: Vec::new(),
            input,
            config,
        }
    }

    /// Pushes the token Inside the input between the start and end indices.
    /// The end index is exclusive.   
    #[inline]
    fn emit(&mut self, ty: TokenType, start: usize, end: usize) {
        self.tokens.push(Token::new(ty, start, end));
    }

    /// Pushes the token whose first character is at the current position
    /// and has the given length.
    #[inline]
    fn emit_cur(&mut self, tape: Tape, ty: TokenType, len: usize) {
        self.tokens
            .push(Token::new(ty, tape.pos, tape.pos + len));
    }

    /// Pops the top of the unclosed pair stack
    /// and pushes the pair whose opener and closer are at the given positions. 
    fn close_pair(&mut self, open: usize, close: usize) {
        self.unclosed_pairs.pop();
        self.pairs.push((open, close));
    }

    /// Attempts to emit a token if the character cluster
    /// belongs to a flanking token, such as an inline format or link.
    ///
    /// `start` is passed to determine the first character of the cluster.
    /// The current position should be the last character in the cluster.
    /// Returns `None` if a token was not emitted.
    fn try_emit_flank(&mut self, tape: Tape<'a>, start: usize, len: usize, ty: TokenType) -> Option<Tape<'a>> {
        if tape.is_l_clear(start) && !tape.is_r_clear(tape.pos) {   // open
            self.unclosed_pairs.push((ty, start));
            return Some(tape);
        } else if tape.is_r_clear(start)
            && self.unclosed_pairs.last().is_some_and(|(t, _)| *t == ty)
        {   // close
            self.pairs
                .push((self.unclosed_pairs.pop().unwrap().1, start + len));
            self.emit_cur(tape, ty, 1);
            return Some(tape);
        }
        None
    }

    /// Resolves all opener-closer pairs in the input and
    /// stores their positions in the `pairs` field.
    /// 
    /// todo
    /// design pattern here
    /// closer, opener terminology
    /// what is a "character cluster"?
    /// what is clearance?
    pub fn pass_1(&mut self, mut tape: Tape<'a>) {
        // Because these symbols may show up in prose,
        // we should expect them to most likely be plain text first
        while let Some(&ch) = self.input.get(tape.pos) {
            let next_tape: Option<Tape<'a>> = match ch {
                b'=' => self.handle_equals(tape),
                b'\\' => self.handle_bslash(tape),
                b'*' => self.handle_star(tape),
                b'`' => self.handle_btick(tape),
                b'$' => self.handle_dollar(tape),
                b'-' => self.handle_dash(tape),
                b'.' => self.handle_dot(tape),
                b'[' => self.handle_brac(tape),
                b'#' => {
                    tape.skip_to(|ch, _| ch == b'\n'); // comment
                    Some(tape)
                }
                b']' => {
                    // check for pair
                    // if true, emit:
                    // self.push_cur_tok(TokenType::CloseBrac, 1);
                    self.eat_open_par = true;
                }
                b'~' => self.try_emit_flank(tape, tape.pos, 1, TokenType::Underline)
                b'_' => self.try_emit_flank(tape, tape.pos, 1, TokenType::Underline)
                
                _ => None
            };
            if let Some(next) = next_tape {
                tape = next;
            }
            tape.adv();
        }
    }

    /// Resolves whether a '[' character belongs to
    fn handle_brac(&mut self, mut tape: Tape<'a>) -> Option<Tape<'a>> {

        self.unclosed_pairs.push((TokenType::Brac, tape.pos));

    }

    // ONLY FIRST NUMBERING MATTERS (sstart)
    // numerals must be within (0,4000)
    // numbers must be nonzero, fit in u8
    /// Resolves whether a '.' character belongs to an ordered List Item or plain text.
    fn handle_dot(&mut self, mut tape: Tape<'a>) -> Option<Tape<'a>> {
        let prev = tape.peek_rev();
        if prev.is_none() {
            return None; 
        }
        let prev = prev.unwrap();
        let dot_pos = tape.pos;
        if prev.is_roman() {
            tape.skip_to_rev(|ch, _| !ch.is_ascii_alphabetic());
            if !tape.at_first_non_ws() {
                tape.pos = dot_pos;
                return None; 
            }
            let num = unsafe { str::from_utf8_unchecked(&tape.raw[tape.pos..dot_pos]) }.parse();
            if num.is_err() {
                return; // invalid numeral; treat dot as text
            }
            let num: RomanNumeral = num.unwrap();
            if prev.is_ascii_uppercase() {
                // case sameness is guaranteed, see:
                // https://docs.rs/crate/roman-numerals-rs/4.1.0/source/src/lib.rs#15
                num.
            } else {
                
            }
        }
        let ty = match prev {
            b'0'..=b'9' => TokenType::NumItem,
            b'a'..=b'z' => TokenType::LowerItem, // ignoring 'z'
            b'A'..=b'Z' => TokenType::UpperItem, // same
            b'.' => {
                tape.dec();
                TokenType::Continuation
            }
            _ => TokenType::ListItem,
        };
    }

    /// Resolves whether a '-' character belongs to an unordered list item,
    /// a checkbox, or plain text.
    fn handle_dash(&mut self, mut tape: Tape<'a>) -> Option<Tape<'a>> {
        if matches!(tape.peek_rev(), Some(b'o') | Some(b'x')) {    // checkbox
            tape.dec(); // decrement to enable check on line start
            if !tape.at_first_non_ws() {
                return None; 
            }
            self.emit_cur(
                tape,
                TokenType::Checkbox { depth: tape.line_indent(), filled: tape.raw[tape.pos] == b'x' },
                2,
            );
            tape.adv();
            return None; // stop at '-'
        }
        if !tape.at_first_non_ws() {
            return None; 
        }
        self.emit_cur(tape, TokenType::ListItem { depth: tape.line_indent() }, 1);
        Some(tape) // stop at '-'
    }

    /// Resolves whether a '=' character belongs to a heading or plain text.
    fn handle_equals(&mut self, mut tape: Tape<'a>) -> Option<Tape<'a>> {
        if !tape.at_first_non_ws() {
            return None; 
        }
        let start = tape.pos;
        self.pgraph_spacing = 1; // ensures we stay on same line
        tape.adv(); // skip first '='
        tape.skip_to_in_pgraph(1, |ch, _| ch != b'=');
        self.pgraph_spacing = 2;
        let depth = tape.pos - start;
        if depth > TokenType::HEADING_MAX {
            return Some(tape); // treat as text, but skip next few '='
        }
        self.emit(TokenType::Heading { depth: depth as u8 }, start, tape.pos);
        tape.dec();
        Some(tape) // stop at final '='
    }

    /// Resolves whether a '$' character belongs to inline math,
    /// a dollar sign literal (if enabled), or plain text.
    /// todo multiline
    fn handle_dollar(&mut self, mut tape: Tape<'a>) -> Option<Tape<'a>> {
        let start = tape.pos;
        let spacing = self.pgraph_spacing;
        if !self.config.handle_inline_math {    
            return None;
        }
        if !tape.try_skip_to_in_pgraph(spacing, |ch, _| ch == b'$') { // failed lookahead
            return None; // stop at '$'
        }
        let body = unsafe { String::from_utf8_unchecked(tape.raw[start + 1..tape.pos].to_vec()) };
        self.tokens
            .push(Token::new(TokenType::InlineMath { body }, start, tape.pos + 1));
        Some(tape)  // stop at closing '$'
    }

    /// Resolves whether a ` character belongs to inline code or plain text.
    /// todo multiline
    fn handle_btick(&mut self, mut tape: Tape<'a>) -> Option<Tape<'a>> {
        let start = tape.pos;
        let spacing = self.pgraph_spacing;
        if tape.at(b"```") {
            return Some(());
        }
        if tape.at(b"``") {
            tape.adv(); // skip over first '`' of open
            if !tape.try_skip_to_in_pgraph(spacing, |_, pos| tape.raw[pos..].starts_with(b"``")) {
                return Some(tape); // stop at 2nd '`'; treat as text
            }
            let body = unsafe { String::from_utf8_unchecked(tape.raw[start + 2..tape.pos].to_vec()) };
            tape.adv(); // skip over first '`' of closer
            self.emit(TokenType::InlineRawCode { body }, start, tape.pos + 1);
            return Some(tape);
        }
        if !tape.try_skip_to_in_pgraph(spacing, |ch, _| ch == b'`') {   // failed lookahead
            return None; // stop at '`'
        }
        let body = unsafe { String::from_utf8_unchecked(tape.raw[start + 1..tape.pos].to_vec()) };
        self.tokens
            .push(Token::new(TokenType::InlineCode { body }, start, tape.pos + 1));
        Some(tape)  // stop at closing '`'
    }

    /// Resolves whether a `*` character belongs to a bold token,
    /// an italic token, both, or plain text.
    fn handle_star(&mut self, mut tape: Tape<'a>) -> Option<Tape<'a>> {
        let start = tape.pos;
        if tape.at(b"***") {
            if !self.try_emit_flank(tape, start, 3, TokenType::ItalicBold)
                && let Some(&(ty, pos)) = self.unclosed_pairs.last()
                && tape.is_any_clear(start)
            {
                if ty == TokenType::Bold {
                    self.close_pair(pos, tape.pos + 2);
                    tape.adv();
                    // stop at 2nd '*'; evaluate single '*' on next iteration
                } else if ty == TokenType::Italic {
                    self.close_pair(pos, tape.pos + 1);
                    // stop at 1st '*'; evaluate '**' on next iteration
                }
            }
        } else if tape.at(b"**") {
            if self.try_emit_flank(tape, start, 2, TokenType::Bold) {
                self.emit_cur(tape, TokenType::Bold, 2);
            }
            None
        } else if self.try_emit_flank(tape, start, 1, TokenType::Italic) {
            self.emit_cur(tape, TokenType::Italic, 1);
        } else {
            None
        }
    }

    /// Resolves whether a `\` character
    /// belongs to an escape character, a macro, or plain text.
    fn handle_bslash(&mut self, mut tape: Tape<'a>) -> Option<Tape<'a>> {
        if tape.pos == tape.raw.len() - 1 { 
            return None;
        }
        let start = tape.pos; // keep for macro handle token
        tape.adv(); // skip past '\'
        let after_bslash = tape.pos;
        tape.skip_to(|ch, _| !ch.is_ascii_alphabetic());
        let first_non_alpha = tape.pos;
        if first_non_alpha == after_bslash {    // treat as escape
            return Some(tape); // stop at the character after '\'
        }
        let spacing = self.pgraph_spacing;
        tape.skip_to_in_pgraph(spacing, |ch, _| !ch.is_ws());
        let mut first_non_ws = tape.cur();
        let fnw_pos = tape.pos;
        if first_non_ws.is_none_or(|ch| ch != b'[' && ch != b'{') { // treat as incomplete macro
            return Some(tape); // stop at the first non-WS character after the macro name
        }
        self.tokens
            .push(Token::new(TokenType::MacroHandle, start, first_non_alpha));
        if first_non_ws == Some(b'[') {
            if !tape.try_skip_to(|ch, _| ch == b']') {  // treat as incomplete macro
                return Some(tape); // stop at '['
            }
            tape.adv(); // skip past ']'
            self.emit(TokenType::MacroArgs, fnw_pos, tape.pos);
            tape.skip_to_in_pgraph(spacing, |ch, _| !ch.is_ws());
            first_non_ws = tape.cur();
            // stop at the next non-WS character after the closing bracket
        }
        if first_non_ws == Some(b'{') {
            if !tape.try_skip_to(|ch, _| ch == b'}') {  // treat as incomplete macro
                return Some(tape); // stop at '{'
            }
            tape.adv(); // skip past '}'
            self.emit(TokenType::MacroBody, fnw_pos, tape.pos);
            // stop at '}'
        }
        Some(tape)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to initialize a parser with byte slices
    fn init_parser<'a>(input: &'a str) -> Parser<'a> {
        Parser::new(
            &ParserConfig {
                handle_inline_math: true,
            },
            input.as_bytes(),
        )
    }

    #[test]
    fn test_heading_resolution() {
        let mut p = init_parser("=== Heading Level 3");
        p.pass_1();

        // Assert that we found 1 token and it's the correct level
        assert_eq!(p.tokens.len(), 1);
        // Assuming your HEAD_TOK_TYPES mapping works:
        // Level 3 should correspond to your H3 TokenType
    }

    #[test]
    fn test_inline_code_block() {
        let mut p = init_parser("`code block` and more text");
        p.pass_1();

        // Check if the token was captured
        assert!(p.tokens.iter().any(|t| t.ty == TokenType::InlineCode));
    }

    #[test]
    fn test_macro_resolution() {
        let mut p = init_parser("\\bold{text}");
        p.pass_1();

        // This validates your resolve_bslash logic
        // Should find MacroHandle and MacroBody
        println!("{:?}", p.tokens);
        assert_eq!(p.tokens.len(), 2);
    }
}
