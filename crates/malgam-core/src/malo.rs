use std::collections::HashMap;
use std::fmt::Display;
use std::num::ParseFloatError;

use thiserror::Error;

use crate::compile::Compile;
use crate::ext::CharExt;
use crate::tape::Tape;

pub type MaloResult = Result<MaloValue, MaloError>;

/// An instance of an `malo` data type.
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
/// 
/// Keys can be
#[derive(Debug, Clone, PartialEq)]
pub enum MaloValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    List(Vec<MaloValue>),
    Object(HashMap<String, MaloValue>),
}

impl Display for MaloValue {
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

impl MaloValue {
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
pub enum MaloError {
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
pub struct Malo<'a> {
    /// The input text.
    pub input: &'a [u8],

    compiled: bool,
    value: MaloResult,
}

impl<'a> Compile for Malo<'a> {
    type Output = MaloResult;

    fn compile(&mut self) -> Self::Output {
        if self.compiled {
            return self.value.clone();
        }
        let val = self.parse_any(&mut Tape::new(self.input));
        self.compiled = true;
        val
    }

    fn is_compiled(&self) -> bool {
        self.compiled
    }
}

impl<'a> Malo<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            compiled: false,
            value: Ok(MaloValue::Null),
        }
    }

    fn parse_any(&mut self, tape: &mut Tape<'a>) -> MaloResult {
        let start = tape.pos;

        // trivial cases
        if tape.cur().is_none() {
            return Err(MaloError::MissingValue { pos: start });
        }
        if tape.is_at(b"true") {
            return Ok(MaloValue::Bool(true));
        }
        if tape.is_at(b"false") {
            return Ok(MaloValue::Bool(false));
        }
        if tape.is_at(b"null") {
            return Ok(MaloValue::Null);
        }

        let ch = tape.cur().unwrap();
        match ch {
            b'.' => self.parse_obj(tape),
            b'{' => self.parse_list(tape),
            b'"' => {
                if !tape.seek_at_in_pgraph(1, b"\"") {
                    Err(MaloError::MissingCloser { open: b'"', close: b'"', open_pos: start })
                } else {
                    Ok(MaloValue::String( unsafe { tape.to_uf8_unchecked() }[start..tape.pos].to_string()))
                }
            },
            b'\'' => {
                if !tape.seek_at_in_pgraph(1, b"'") {
                    Err(MaloError::MissingCloser { open: b'\'', close: b'\'', open_pos: start })
                } else {
                    Ok(MaloValue::String( unsafe { tape.to_uf8_unchecked() }[start..tape.pos].to_string()))
                }
            }
            b'0'..=b'9' =>                 unsafe { tape.to_uf8_unchecked().parse::<f64>() }
                    .map(|n| MaloValue::Number(n))
                    .map_err(|e| MaloError::InvalidNumber(e)),
            b';' => {   // same comment style as Malgam
                Err(MaloError::MissingValue { pos: start })
            }
            _ => Err(MaloError::IllegalCharacter { ch, pos: start })
        }
    }

    fn parse_obj(&mut self, tape: &mut Tape<'a>) -> MaloResult {
        tape.adv(); // skip '.'
        if tape.cur() != Some(b'{') {   // should not be checked beforehand
            return Err(MaloError::IllegalCharacter { ch: tape.cur().unwrap_or(0), pos: tape.pos });
        }
        let open_pos = tape.pos;
        tape.adv(); // skip '{'
        tape.consume(|ch,_| ch.is_hg_ws());
        let mut map = HashMap::new();
        loop {  // allows leading, trailing, and mixed/chained delimiters
            tape.consume(|ch,_| ch.is_hg_ws() || ch == b'\n' || ch == b',');

            // get current character
            let ch = tape.cur();
            if ch.is_none() {
                return Err(MaloError::MissingCloser { open: b'{', close: b'}', open_pos })
            }
            let ch = ch.unwrap();

            if ch == b'}' {
                tape.adv();
                break;
            }

            // get key
            let key: &'a [u8];
            let raw = tape.raw;
            if ch == b'"' {
                tape.adv();
                key = tape.consume(|ch, pos| (ch != b'"' && ch != b'\n') || ch == b'"' && raw.get(pos - 1) == Some(&b'\\'));
                tape.adv(); // skip '"'
            }
            else if ch == b'\'' {
                tape.adv();
                 key = tape.consume(|ch, pos| (ch != b'\'' && ch != b'\n') || ch == b'\'' && raw.get(pos - 1) == Some(&b'\\'));
                tape.adv(); // skip `'`
            }
            else {
                key = tape.consume(|ch, _| ch.is_hgon_key_part());
            }
            if key.is_empty() {
                return Err(MaloError::MissingValue { pos: tape.pos });
            }
            let key = unsafe { str::from_utf8_unchecked(key) }.to_string();

            tape.consume(|ch,_| ch.is_hg_ws());
            if tape.cur() != Some(b'=') {
                return Err(MaloError::IllegalCharacter { ch: tape.cur().unwrap_or(0), pos: tape.pos });
            }
            tape.adv(); // skip '='
            tape.consume(|ch,_| ch.is_hg_ws());
            let val = self.parse_any(tape)?;
            map.insert(key, val);
        } 
        Ok(MaloValue::Object(map))
    }

    fn parse_list(&mut self, tape: &mut Tape<'a>) -> MaloResult {
        let mut items = Vec::new();
        loop {
            tape.consume(|ch,_| ch.is_hg_ws() || ch == b'\n');
            if tape.cur() == Some(b'}') {
                tape.adv();
                break;
            }
            if tape.cur().is_none() {
                return Err(MaloError::MissingCloser { open: b'{', close: b'}', open_pos: tape.pos });
            }
            let val = self.parse_any(tape)?;
            items.push(val);
            tape.consume(|ch,_| ch.is_hg_ws() || ch == b'\n');
            if tape.cur() == Some(b',') {
                tape.adv();
            } else if tape.cur() != Some(b'}') {
                return Err(MaloError::IllegalCharacter { ch: tape.cur().unwrap_or(0), pos: tape.pos });
            }
        }
        Ok(MaloValue::List(items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sample_config() {
        let content = b".{\n    my-paragraph =\n        | this is\n        | a type of\n        | multiline string\n        ,\n    finance-mode = true,\n}// Trailing Commas (The RSI Savior)\n";
        let parsed = Malo::new(content).compile().expect("parse config.mgon");

        let mut expected = HashMap::new();
        expected.insert(
            "my-paragraph".to_string(),
            MaloValue::String("this is\na type of\nmultiline string".to_string()),
        );
        expected.insert("finance-mode".to_string(), MaloValue::Bool(true));

        assert_eq!(parsed, MaloValue::Object(expected));
    }

    #[test]
    fn parse_object_with_numbers_and_strings() {
        let content = b"{ count = 42, pi = 3.14, name = 'mgon', active = false }";
        let parsed = Malo::new(content).compile().expect("parse mgon object");

        let mut expected = HashMap::new();
        expected.insert("count".into(), MaloValue::Number(42.0));
        expected.insert("pi".into(), MaloValue::Number(3.14));
        expected.insert("name".into(), MaloValue::String("mgon".into()));
        expected.insert("active".into(), MaloValue::Bool(false));

        assert_eq!(parsed, MaloValue::Object(expected));
    }
}