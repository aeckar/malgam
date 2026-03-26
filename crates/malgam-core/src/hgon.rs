use std::collections::HashMap;
use std::fmt::Display;
use std::num::ParseFloatError;
use std::str;

use thiserror::Error;

use crate::compile::Compile;
use crate::ext::CharExt;
use crate::tape::Tape;

pub type HgonResult = Result<HgonValue, HgonError>;

/// An instance of an `hgon` data type.
/// 
/// Roughly reflects JSON data types. Numbers **must** start with a digit.
/// Unlike standard JSON, allows for trailing commas.
/// 
/// All numbers follow  IEEE 754 64-bit floating-point format, including
/// the infinities (`inf|infinity|+inf|+infinity|-inf|-infinity`) and not-a-number
/// (`nan`, case insensitive).
/// 
/// Strings may be enclosed using either `'` or `"`.
/// 
/// The `fmt` (and as a result, `to_string`) implementations emit the
/// most concise object notation possible. Pretty printing is supported via the
/// `pfmt` and `to_pstring` functions. Strings are always enclosed using `"`.
#[derive(Debug, Clone, PartialEq)]
pub enum HgonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    List(Vec<HgonValue>),
    Object(HashMap<String, HgonValue>),
}

impl Display for HgonValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(cond) => write!(f, "{cond}"),
            Self::Number(n) => write!(f, "{n}"),
                        Self::String(str) => write!(f, "\"{str}\""),
            Self::List(items) => {
                write!(f, "{{");
                for item in items {
                    write!(f, "{item}");
                    write!(f, ",");
                }
                write!(f, "}}")
            },
            Self::Object(map) => {
                write!(f, ".{{");
                for (key, val) in map {
                    write!(f, "{key}:{val}");
                    write!(f, ",");
                }
                write!(f, "}}")
            },
        };
        Ok(())
    }
}

impl HgonValue {
    pub fn to_pstring(&self) -> String {
        let mut buf = String::new();
        // Start with 0 indentation
        self.pfmt(&mut buf, 0).unwrap();
        buf
    }

    pub fn pfmt(&self, f: &mut dyn std::fmt::Write, indent: usize) -> std::fmt::Result {
        let space = " ".repeat(indent * 4); // 4-space indent
        let next_space = " ".repeat((indent + 1) * 4);

        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Number(n) => write!(f, "{n}"),
            Self::String(s) => write!(f, "\"{s}\""),
            Self::List(items) => {
                if items.is_empty() {
                    return write!(f, "{{}}");
                }
                writeln!(f, "{{")?;
                for item in items {
                    write!(f, "{next_space}")?;
                    item.pfmt(f, indent + 1)?;
                    writeln!(f, ",")?;
                }
                write!(f, "{space}}}")
            }
            Self::Object(map) => {
                if map.is_empty() {
                    return write!(f, ".{{}}");
                }
                writeln!(f, ".{{")?;
                for (key, val) in map {
                    write!(f, "{next_space}\"{key}\": ")?;
                    val.pfmt(f, indent + 1)?;
                    writeln!(f, ",")?;
                }
                write!(f, "{space}}}")
            }
        }
    }
}

/// Describes and locates a specific error in `hgon` syntax. 
#[derive(Error, Debug, Clone)]
pub enum HgonError {
    #[error("Expected a value at index {pos}")]
    MissingValue { pos: usize },

    #[error("Number cannot be parsed {_0}")]
    InvalidNumber(#[from] ParseFloatError),

    #[error("Illegal character '{ch}' at index {pos}")]
    IllegalCharacter { ch: u8, pos: usize },

    #[error("Expected a closing '{close}' for '{open}' at {open_pos}")]
    MissingCloser { open: u8, close: u8, open_pos: usize }
}

/// Malgam Object Notation (HGON) syntax.
pub struct Hgon<'a> {
    /// The input text.
    pub input: &'a [u8],

    compiled: bool,
    value: HgonResult,
}

impl<'a> Compile for Hgon<'a> {
    type Output = HgonResult;

    fn compile(&mut self) -> Self::Output {
        if self.compiled {
            return self.value.clone();
        }
        let val = self.parse_any(Tape::new(self.input));
        self.compiled = true;
        val
    }

    fn is_compiled(&self) -> bool {
        self.compiled
    }
}

impl<'a> Hgon<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            compiled: false,
            value: Ok(HgonValue::Null),
        }
    }

    fn parse_any(&mut self, mut tape: Tape<'a>) -> HgonResult {
        let start = tape.pos;
        if tape.cur().is_none() {
            return Err(HgonError::MissingValue { pos: start });
        }
        if tape.is_at(b"true") {
            return Ok(HgonValue::Bool(true));
        }
        if tape.is_at(b"false") {
            return Ok(HgonValue::Bool(false));
        }
        if tape.is_at(b"null") {
            return Ok(HgonValue::Null);
        }
        let ch = tape.cur().unwrap();
        match ch {
            b'.' => self.parse_obj(tape),
            b'{' => self.parse_list(tape),
            b'"' => {
                if !tape.seek_at_in_pgraph(1, b"\"") {
                    Err(HgonError::MissingCloser { open: b'"', close: b'"', open_pos: start })
                } else {
                    Ok(HgonValue::String( unsafe { tape.to_uf8_unchecked() }[start..tape.pos].to_string()))
                }
            },
            b'\'' => {
                if !tape.seek_at_in_pgraph(1, b"'") {
                    Err(HgonError::MissingCloser { open: b'\'', close: b'\'', open_pos: start })
                } else {
                    Ok(HgonValue::String( unsafe { tape.to_uf8_unchecked() }[start..tape.pos].to_string()))
                }
            }
            b'0'..=b'9' =>                 unsafe { tape.to_uf8_unchecked().parse::<f64>() }
                    .map(|n| HgonValue::Number(n))
                    .map_err(|e| HgonError::InvalidNumber(e)),
            b';' => {   // same comment style as Malgam
                Err(HgonError::MissingValue { pos: start })
            }
            _ => Err(HgonError::IllegalCharacter { ch, pos: start })
        }
    }

fn parse_obj(&mut self, mut tape: Tape<'a>) -> HgonResult {
        if tape.cur() != Some(b'{') {
            return Err(HgonError::IllegalCharacter { ch: tape.cur().unwrap_or(0), pos: tape.pos });
        }
        tape.adv(); // skip '{'

        let mut map = std::collections::HashMap::new();

        loop {
            tape.seek(|ch,_| !ch.is_flank_ws());
            if tape.cur() == Some(b'}') {
                tape.adv();
                break;
            }

            // 1. Parse Key (assuming keys are unquoted or strings)
            let key_slice = tape.consume(|c, _| c.is_ascii_alphanumeric() || c == b'_');
            if key_slice.is_empty() {
                 return Err(HgonError::MissingValue { pos: tape.pos });
            }
            let key = unsafe { std::str::from_utf8_unchecked(key_slice) }.to_string();

            tape.seek(|ch,_| !ch.is_flank_ws());
            if tape.cur() != Some(b':') {
                return Err(HgonError::IllegalCharacter { ch: tape.cur().unwrap_or(0), pos: tape.pos });
            }
            tape.adv(); // skip ':'

            // 2. Parse Value
            let val = self.parse_any(tape)?;
            map.insert(key, val);

            // 3. Handle Delimiters
            tape.seek(|ch,_| !ch.is_flank_ws());
            if tape.cur() == Some(b',') {
                tape.adv();
            }
        }

        Ok(HgonValue::Object(map))
    }

    fn parse_list(&mut self, mut tape: Tape<'a>) -> HgonResult {
        let mut items = Vec::new();

        loop {
            tape.seek(|ch,_| !ch.is_flank_ws());
            if tape.cur() == Some(b'}') {
                tape.adv();
                break;
            }

            if tape.cur().is_none() {
                return Err(HgonError::MissingCloser { open: b'{', close: b'}', open_pos: tape.pos });
            }

            let val = self.parse_any(tape)?;
            items.push(val);

            tape.seek(|ch,_| !ch.is_flank_ws());
            if tape.cur() == Some(b',') {
                tape.adv();
            } else if tape.cur() != Some(b'}') {
                // If no comma and no closer, it's a syntax error
                return Err(HgonError::IllegalCharacter { ch: tape.cur().unwrap_or(0), pos: tape.pos });
            }
        }

        Ok(HgonValue::List(items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sample_config() {
        let content = b".{\n    my-paragraph =\n        | this is\n        | a type of\n        | multiline string\n        ,\n    finance-mode = true,\n}// Trailing Commas (The RSI Savior)\n";
        let parsed = Hgon::parse(content).expect("parse config.mgon");

        let mut expected = HashMap::new();
        expected.insert(
            "my-paragraph".to_string(),
            HgonValue::String("this is\na type of\nmultiline string".to_string()),
        );
        expected.insert("finance-mode".to_string(), HgonValue::Bool(true));

        assert_eq!(parsed, HgonValue::Object(expected));
    }

    #[test]
    fn parse_object_with_numbers_and_strings() {
        let content = b"{ count = 42, pi = 3.14, name = 'mgon', active = false }";
        let parsed = Hgon::parse(content).expect("parse mgon object");

        let mut expected = HashMap::new();
        expected.insert("count".into(), HgonValue::Number(42.0));
        expected.insert("pi".into(), HgonValue::Number(3.14));
        expected.insert("name".into(), HgonValue::String("mgon".into()));
        expected.insert("active".into(), HgonValue::Bool(false));

        assert_eq!(parsed, HgonValue::Object(expected));
    }
}