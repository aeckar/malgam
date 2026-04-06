use std::collections::HashMap;
use std::fmt::Display;
use std::num::ParseFloatError;
use std::str::Utf8Error;
use std::string::FromUtf8Error;

use thiserror::Error;

use crate::compile::Compile;
use crate::ext::{CharExt, SliceExt};
use crate::tape::Tape;

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
pub enum ObjectValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    List(Vec<ObjectValue>),
    Object(HashMap<String, ObjectValue>),
}

impl Display for ObjectValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(cond) => write!(f, "{cond}"),
            Self::Number(n) => write!(f, "{n}"),
                        Self::String(str) => write!(f, "\"{str}\""),
            Self::List(items) => {
                write!(f, "{{")?;
                for item in items {
                    write!(f, "{item}")?;
                    write!(f, ",")?;
                }
                write!(f, "}}")
            },
            Self::Object(map) => {
                write!(f, ".{{")?;
                for (key, val) in map {
                    write!(f, "{key}:{val}")?;
                    write!(f, ",")?;
                }
                write!(f, "}}")
            },
        };
        Ok(())
    }
}

impl ObjectValue {
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

/// Describes and locates a specific error in object notation syntax. 
#[derive(Error, Debug, Clone)]
pub enum ObjectError {
    #[error("Expected a value at index {pos}")]
    MissingValue { pos: usize },

    #[error("Number cannot be parsed {_0}")]
    InvalidNumber(#[from] ParseFloatError),

    #[error("Illegal character '{ch}' at index {pos}")]
    IllegalCharacter { ch: u8, pos: usize },

    #[error("Invalid UTF-8")]
    InvalidUtf8(#[from] Utf8Error),

    #[error("Expected a closing '{close}' for '{open}' at {open_pos}")]
    MissingCloser { open: u8, close: u8, open_pos: usize }
}

/// Draft Object Notation (DON) syntax.
pub struct ObjectFile<'a> {
    /// The input text.
    pub input: &'a [u8],

    compiled: bool,
    value: Result<ObjectValue, ObjectError>,
}

impl<'a> Compile for ObjectFile<'a> {
    type Output = Result<ObjectValue, ObjectError>;

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

impl<'a> ObjectFile<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            compiled: false,
            value: Ok(ObjectValue::Null),
        }
    }

    fn parse_any(&mut self, tape: &mut Tape<'a>) -> Result<ObjectValue, ObjectError> {
        let start = tape.pos;

        // trivial cases
        if tape.cur().is_none() {
            return Err(ObjectError::MissingValue { pos: start });
        }
        if tape.is_at(b"true") {
            return Ok(ObjectValue::Bool(true));
        }
        if tape.is_at(b"false") {
            return Ok(ObjectValue::Bool(false));
        }
        if tape.is_at(b"null") {
            return Ok(ObjectValue::Null);
        }
        // todo inf, nan, +/-

        let ch = tape.cur().unwrap();
        match ch {
            b'.' => self.parse_obj(tape),
            b'{' => self.parse_list(tape),
            b'"' => {
                if !tape.seek_at_in_pgraph(1, b"\"") {
                    Err(ObjectError::MissingCloser { open: b'"', close: b'"', open_pos: start })
                } else {
                    Ok(ObjectValue::String( tape.slice(start + 1..tape.pos).to_utf8()?))
                }
            },
            b'\'' => {
                if !tape.seek_at_in_pgraph(1, b"'") {
                    Err(ObjectError::MissingCloser { open: b'\'', close: b'\'', open_pos: start })
                } else {
                    Ok(ObjectValue::String( tape.slice(start + 1..tape.pos).to_utf8()?))
                }
            }

            b'0'..=b'9' =>                 tape.consume(|ch,_| ch.is_ascii_digit()).to_utf8()?.parse::<f64>() // todo
                    .map(|n| ObjectValue::Number(n))
                    .map_err(|e| ObjectError::InvalidNumber(e)),
            b';' => {   // same comment style as markup
                Err(ObjectError::MissingValue { pos: start })
            }
            _ => Err(ObjectError::IllegalCharacter { ch, pos: start })
        }
    }

    fn parse_obj(&mut self, tape: &mut Tape<'a>) -> Result<ObjectValue, ObjectError> {
        tape.adv(); // skip '.'
        if tape.cur() != Some(b'{') {   // should not be checked beforehand
            return Err(ObjectError::IllegalCharacter { ch: tape.cur().unwrap_or(0), pos: tape.pos });
        }
        let open_pos = tape.pos;
        tape.adv(); // skip '{'
        tape.consume(|ch,_| ch.is_file_ws());
        let mut map = HashMap::new();
        loop {  // allows leading, trailing, and mixed/chained delimiters
            tape.consume(|ch,_| ch.is_file_ws() || ch == b'\n' || ch == b',');

            // get current character
            let ch = tape.cur();
            if ch.is_none() {
                return Err(ObjectError::MissingCloser { open: b'{', close: b'}', open_pos })
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
                key = tape.consume(|ch, _| ch.is_file_key_part());
            }
            if key.is_empty() {
                return Err(ObjectError::MissingValue { pos: tape.pos });
            }
            let key = str::from_utf8(key)?.to_string();

            tape.consume(|ch,_| ch.is_file_ws());
            if tape.cur() != Some(b'=') {
                return Err(ObjectError::IllegalCharacter { ch: tape.cur().unwrap_or(0), pos: tape.pos });
            }
            tape.adv(); // skip '='
            tape.consume(|ch,_| ch.is_file_ws());
            let val = self.parse_any(tape)?;
            map.insert(key, val);
        } 
        Ok(ObjectValue::Object(map))
    }

    fn parse_list(&mut self, tape: &mut Tape<'a>) -> Result<ObjectValue, ObjectError> {
        let mut items = Vec::new();
        loop {
            tape.consume(|ch,_| ch.is_file_ws() || ch == b'\n');
            if tape.cur() == Some(b'}') {
                tape.adv();
                break;
            }
            if tape.cur().is_none() {
                return Err(ObjectError::MissingCloser { open: b'{', close: b'}', open_pos: tape.pos });
            }
            let val = self.parse_any(tape)?;
            items.push(val);
            tape.consume(|ch,_| ch.is_file_ws() || ch == b'\n');
            if tape.cur() == Some(b',') {
                tape.adv();
            } else if tape.cur() != Some(b'}') {
                return Err(ObjectError::IllegalCharacter { ch: tape.cur().unwrap_or(0), pos: tape.pos });
            }
        }
        Ok(ObjectValue::List(items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

}