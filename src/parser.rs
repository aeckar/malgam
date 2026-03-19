//! Don't check for UTF-8 correctness; leave that to the user.

use roman_numerals_rs::RomanNumeral;

use crate::char_ext::CharExt;
use crate::slice_ext::SliceExt;
use crate::tape::Tape;
use crate::{FlankType, NumberingType, Token, TokenType};

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
    /// 
    /// The first element of each pair is the flank type enum.
    unclosed_pairs: Vec<(u8, usize)>,

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

    /// Attempts to emit a token if the character cluster
    /// belongs to a flanking token, such as an inline format or link.
    ///
    /// `start` is passed to determine the first character of the cluster.
    /// The current position should be the last character in the cluster.
    /// Returns `None` if a token was not emitted.
    /// 
    /// If `None` is not returned, the length of `self.unclosed_pairs` is always modified
    /// and the cursor of the returned tape is left at the final character of the cluster.
    #[must_use]
    fn try_emit_flank(&mut self, mut tape: Tape<'a>, start: usize, len: usize, ty: u8) -> Option<Tape<'a>> {
        if tape.is_l_clear(start) && !tape.is_r_clear(tape.pos) {   // open
            self.unclosed_pairs.push((ty, start));
            tape.pos += len - 1;
            return Some(tape);
        } else if tape.is_r_clear(start)
            && self.unclosed_pairs.last().is_some_and(|(t, _)| *t & ty != 0)
        {   // close
            let (open_fty, pair_start) = self.unclosed_pairs.pop().unwrap();
            self.pairs
                .push((pair_start, start + len));
            if ty == FlankType::BOLD | FlankType::ITALIC {
                let open_tty = if open_fty == FlankType::BOLD {
                    TokenType::Bold
                } else {
                    TokenType::Italic
                };
                self.emit_cur(tape, open_tty, len);
                if open_fty == FlankType::BOLD {
                    tape.pos -= 1;  // precede trailing '*'
                } else {
                    tape.pos -= 2;  // precede trailing '**'
                };
            } else {
                // IMPORTANT: assumes u8 bitflags
                self.emit_cur(tape, TokenType::FLANK[open_fty.ilog2() as usize].clone(), len);
            }
            tape.pos += len - 1;
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
    /// returning None Relinquishes the need to reset the position to the start. 
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
                    tape.seek(|ch, _| ch == b'\n'); // comment
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
    #[must_use]
    fn handle_brac(&mut self, mut tape: Tape<'a>) -> Option<Tape<'a>> {

        self.unclosed_pairs.push((TokenType::Brac, tape.pos));

    }

    // ONLY FIRST NUMBERING MATTERS (sstart)
    // IF START WITH CONTINUATION, USE DEFAULT SEQUENCE
    // numerals must be within (0,4000)
    // numbers must be nonzero, fit in u8
    /// Resolves whether a '.' character belongs to an ordered List Item or plain text.
    #[must_use]
    fn handle_dot(&mut self, tape: Tape<'a>) -> Option<Tape<'a>> {
        if tape.is_cur_prefix() {   
            self.emit_cur(tape, TokenType::NumberedItem { depth: tape.count_indent(), ty: NumberingType::Continuation }, 1);
            return Some(tape);
        }
        let prev = tape.peek_back();
        if prev.is_none() || !tape.is_prefix(tape.pos - 1) {
            return None; 
        }
        let ty = match prev.unwrap() {
            b'd' => NumberingType::Number,
            b'a' => NumberingType::Lower,
            b'A' => NumberingType::Upper,
            b'r' => NumberingType::LowerNumeral,
            b'R' => NumberingType::UpperNumeral,
            _ => { return None;}
        };
        self.emit(TokenType::NumberedItem { depth: tape.count_indent(), ty }, tape.pos - 1, tape.pos + 1);
        Some(tape)
    }

    /// Resolves whether a '-' character belongs to an unordered list item,
    /// a checkbox, or plain text.
    #[must_use]
    fn handle_dash(&mut self, mut tape: Tape<'a>) -> Option<Tape<'a>> {
        if matches!(tape.peek_back(), Some(b'o') | Some(b'x')) {    // checkbox
            tape.dec(); // decrement to enable check on line start
            if !tape.is_cur_prefix() {
                return None; 
            }
            self.emit_cur(
                tape,
                TokenType::Checkbox { depth: tape.count_indent(), filled: tape.raw[tape.pos] == b'x' },
                2,
            );
            tape.adv();
            return None; // stop at '-'
        }
        if !tape.is_cur_prefix() {
            return None; 
        }
        self.emit_cur(tape, TokenType::ListItem { depth: tape.count_indent() }, 1);
        Some(tape) // stop at '-'
    }

    /// Resolves whether a '=' character belongs to a heading or plain text.
    #[must_use]
    fn handle_equals(&mut self, mut tape: Tape<'a>) -> Option<Tape<'a>> {
        if !tape.is_cur_prefix() {
            return None; 
        }
        let start = tape.pos;
        let marker = tape.consume_in_pgraph(1, |ch, _| ch == b'=');
        let depth = marker.len();
        if depth > TokenType::HEADING_MAX {
            return Some(tape); // treat as text, but skip next few '='
        }
        self.emit(TokenType::Heading { depth: depth as u8 }, start, tape.pos);
        self.pgraph_spacing = 1;
        tape.dec();
        Some(tape) // stop at final '='
    }

    /// Resolves whether a '$' character belongs to inline math,
    /// a dollar sign literal (if enabled), or plain text.
    /// todo multiline
    #[must_use]
    fn handle_dollar(&mut self, mut tape: Tape<'a>) -> Option<Tape<'a>> {
        let start = tape.pos;
        if !self.config.handle_inline_math {    
            return None;
        }
        if !tape.seek_in_pgraph(self.pgraph_spacing, |ch, _| ch == b'$') { // failed lookahead
            return None; // stop at '$'
        }
        self.tokens
            .push(Token::new(TokenType::InlineMath { body: &tape.raw[start + 1.. tape.pos] }, start, tape.pos + 1));
        Some(tape)  // stop at closing '$'
    }

    /// Resolves whether a ` character belongs to inline code or plain text.
    #[must_use]
    fn handle_btick(&mut self, mut tape: Tape<'a>) -> Option<Tape<'a>> {
        let start = tape.pos;
        let spacing = self.pgraph_spacing;
        if tape.at(b"```") {
            if !tape.is_cur_prefix() {
                return None;
            }
            tape.pos += 3;  // skip over '```'
            let lang = tape.consume(|ch,_| ch != b'\n');
            let body_start = tape.pos + 1;
            if !tape.seek(|_,pos| tape.raw[pos..].starts_with(b"\n```")) { // failed lookahead
                return None;
            }
            tape.pos += 3;  // stop at last '`'
            self.emit(TokenType::CodeBlock { body: &tape.raw[body_start.. tape.pos], lang: lang.trim_ws() }, start, tape.pos + 1);
            return Some(tape);
        }
        if tape.at(b"``") {
            tape.adv(); // skip over first '`' of open
            if !tape.seek_in_pgraph(spacing, |_, pos| tape.raw[pos..].starts_with(b"``")) {
                return Some(tape); // stop at 2nd '`'; treat as text
            }
            tape.adv(); // skip over first '`' of closer
            self.emit(TokenType::InlineRawCode { body: &tape.raw[start + 2.. tape.pos] }, start, tape.pos + 1);
            return Some(tape);
        }
        if !tape.seek_in_pgraph(spacing, |ch, _| ch == b'`') {   // failed lookahead
            return None; // stop at '`'
        }
        self.tokens
            .push(Token::new(TokenType::InlineCode { body: &tape.raw[start + 1.. tape.pos] }, start, tape.pos + 1));
        Some(tape)  // stop at closing '`'
    }

    /// Resolves whether a `*` character belongs to a bold token,
    /// an italic token, both, or plain text.
    #[must_use]
    fn handle_star(&mut self, tape: Tape<'a>) -> Option<Tape<'a>> {
        let start = tape.pos;
        if tape.at(b"***") {
            self.try_emit_flank(tape, start, 3, FlankType::BOLD | FlankType::ITALIC)
        } else if tape.at(b"**") {
            self.try_emit_flank(tape, start, 2, FlankType::BOLD)
        } else {    // try for '*'
            self.try_emit_flank(tape, start, 1, FlankType::ITALIC)
        }
    }

    /// Resolves whether a `\` character
    /// belongs to an escape character, a macro, or plain text.
    #[must_use]
    fn handle_bslash(&mut self, mut tape: Tape<'a>) -> Option<Tape<'a>> {
        if tape.pos == tape.raw.len() - 1 { 
            return None;
        }
        let start = tape.pos; // keep for macro handle token
        tape.adv(); // skip past '\'
        let name = tape.consume(|ch, _| ch.is_ascii_alphabetic());
        if name.len() == 0 {    // treat as escape
            return Some(tape); // stop at the character after '\'
        }
        tape.consume_in_pgraph(self.pgraph_spacing, |ch, _| ch.is_ws());
        let first_non_ws = tape.pos;
        let mut next = tape.cur();
        if next.is_none_or(|ch| ch != b'[' && ch != b'{') { // treat as incomplete macro
            return Some(tape); // stop at the first non-WS character after the macro name
        }
        self.tokens
            .push(Token::new(TokenType::MacroHandle { name}, start, start + name.len() + 1));
        if next == Some(b'[') {
            if !tape.seek(|ch, _| ch == b']') {  // treat as incomplete macro
                return Some(tape); // stop at '['
            }
            tape.adv(); // skip past ']'
            self.emit(TokenType::MacroArgs {body: &tape.raw[first_non_ws + 1..tape.pos]}, first_non_ws, tape.pos);
            next = tape.cur();
            // stop after ']'
        }
        while next == Some(b'{') {
            if !tape.seek(|ch, _| ch == b'}') {  // treat as incomplete macro
                return Some(tape); // stop at '{'
            }
            tape.adv(); // skip past '}'
            self.emit(TokenType::MacroBody{ body:&tape.raw[first_non_ws+1..tape.pos]}, first_non_ws, tape.pos);
            // stop after '}'
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
}
