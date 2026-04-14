use std::collections::HashMap;
use std::fmt::Display;
use std::str::Utf8Error;

use thiserror::Error;

use crate::ext::{CharExt, SliceExt};
use crate::markup::Compile;
use crate::tape::Tape;

/// An instance of a Draft object.
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
pub enum DataValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    List(Vec<DataValue>),
    Object {
        tag: String,
        map: HashMap<String, DataValue>,
    },
}

impl Display for DataValue {
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
            }
            Self::Object { tag, map } => {
                write!(f, "{tag}.{{")?;
                for (key, val) in map {
                    write!(f, "{key}:{val}")?;
                    write!(f, ",")?;
                }
                write!(f, "}}")
            }
        };
        Ok(())
    }
}

impl DataValue {
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
            Self::Object { tag, map } => {
                if map.is_empty() {
                    return write!(f, "{tag}.{{}}");
                }
                writeln!(f, "{tag}.{{")?;
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
pub enum DataError {
    #[error("Expected a value at index {pos}")]
    MissingValue { pos: usize },

    #[error("Illegal character '{ch}' at index {pos}")]
    IllegalCharacter { ch: u8, pos: usize },

    #[error("{_0}")]
    InvalidNumber(lexical_core::Error),

    #[error("Invalid UTF-8")]
    InvalidUtf8(#[from] Utf8Error),

    #[error("Expected a closing '{close}' for '{open}' at {open_pos}")]
    MissingCloser {
        open: u8,
        close: u8,
        open_pos: usize,
    },
}

/// Draft Object Notation (DON) syntax.
pub struct DataFile<'a> {
    /// The input text.
    pub input: &'a [u8],
}

impl<'a> Compile for DataFile<'a> {
    type Output = Result<DataValue, DataError>;

    fn compile(self) -> Self::Output {
        self.parse_any(&mut Tape::new(self.input))
    }
}

/// All `parse_X` functions assume cursor is at a valid character.
impl<'a> DataFile<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
        }
    }

    fn parse_any(&self, tape: &mut Tape<'a, u8>) -> Result<DataValue, DataError> {
        use lexical_core::format;
        use lexical_core::parse_float_options as options;
        const NUM_FMT: u128 = format::STANDARD;
        const NUM_OPTIONS: options::Options = options::Options::builder()
            .decimal_point(b'.')
            .inf_string(Some(b"inf"))
            .infinity_string(Some(b"infinity"))
            .exponent(b'e')
            .lossy(false) // greater accuracy, slower on precise numbers
            .nan_string(Some(b"nan"))
            .build_strict();

        let start = tape.pos;

        // trivial cases
        if tape.cur().is_none() {
            return Err(DataError::MissingValue { pos: start });
        }
        if tape.is_at(b"true") {
            return Ok(DataValue::Bool(true));
        }
        if tape.is_at(b"false") {
            return Ok(DataValue::Bool(false));
        }
        if tape.is_at(b"null") {
            return Ok(DataValue::Null);
        }

        let ch = tape.cur().unwrap();
        match ch {
            b'$' | b'a'..=b'z' | b'A'..=b'Z' => {
                let tag = self.parse_tag(tape)?;
                self.parse_obj(tape, tag)
            }
            b'.' => self.parse_obj(tape, "".to_string()),
            b'{' => self.parse_list(tape),
            b'"' => {
                if !tape.seek_at_in_pgraph(1, b"\"") {
                    Err(DataError::MissingCloser {
                        open: b'"',
                        close: b'"',
                        open_pos: start,
                    })
                } else {
                    Ok(DataValue::String(
                        tape.slice(start + 1..tape.pos).to_utf8()?,
                    ))
                }
            }
            b'\'' => {
                if !tape.seek_at_in_pgraph(1, b"'") {
                    Err(DataError::MissingCloser {
                        open: b'\'',
                        close: b'\'',
                        open_pos: start,
                    })
                } else {
                    Ok(DataValue::String(
                        tape.slice(start + 1..tape.pos).to_utf8()?,
                    ))
                }
            }
            b'-' | b'+' | b'0'..=b'9' => lexical_core::parse_partial_with_options::<f64, NUM_FMT>(
                tape.slice(tape.pos..tape.raw.len()),
                &NUM_OPTIONS,
            )
            .inspect(|&(_, len)| tape.pos += len)
            .map(|(n, _)| DataValue::Number(n))
            .map_err(|e| DataError::InvalidNumber(e)),
            b';' => {
                // same comment style as markup
                Err(DataError::MissingValue { pos: start })
            }
            _ => Err(DataError::IllegalCharacter { ch, pos: start }),
        }
    }

    fn parse_tag(&self, tape: &mut Tape<'a, u8>) -> Result<String, DataError> {
        let tag = tape.consume(|ch, _| ch.is_file_key_part());
        let tag = &tag[..tag.len() - 1]; // safe; first character already seen
        if tape.cur() != Some(b'{') {
            let pos = tape.pos;
            return Err(DataError::IllegalCharacter { ch: tape[pos], pos });
        }
        tape.dec(); // put back '.'
        Ok(str::from_utf8(tag)?.to_string())
    }

    fn parse_obj(&self, tape: &mut Tape<'a, u8>, tag: String) -> Result<DataValue, DataError> {
        tape.adv(); // skip '.'
        if tape.cur() != Some(b'{') {
            // should not be checked beforehand
            return Err(DataError::IllegalCharacter {
                ch: tape.cur().unwrap_or(0),
                pos: tape.pos,
            });
        }
        let open_pos = tape.pos;
        tape.adv(); // skip '{'
        tape.consume(|ch, _| ch.is_file_ws());
        let mut map = HashMap::new();
        loop {
            // Allow leading, trailing, and mixed/chained delimiters
            tape.consume(|ch, _| ch.is_file_ws() || ch == b'\n' || ch == b',');

            // Get current character
            let ch = tape.cur();
            if ch.is_none() {
                return Err(DataError::MissingCloser {
                    open: b'{',
                    close: b'}',
                    open_pos,
                });
            }
            let ch = ch.unwrap();

            // Check if end is reached
            if ch == b'}' {
                tape.adv();
                break;
            }

            // Get key
            let key: &'a [u8];
            let raw = tape.raw;
            if ch == b'"' {
                tape.adv();
                key = tape.consume(|ch, pos| {
                    (ch != b'"' && ch != b'\n') || ch == b'"' && raw.get(pos - 1) == Some(&b'\\')
                });
                tape.adv(); // skip '"'
            } else if ch == b'\'' {
                tape.adv();
                key = tape.consume(|ch, pos| {
                    (ch != b'\'' && ch != b'\n') || ch == b'\'' && raw.get(pos - 1) == Some(&b'\\')
                });
                tape.adv(); // skip `'`
            } else if matches!(ch, b'-' | b'$' | b'a'..=b'z' | b'A'..=b'Z') {
                key = tape.consume(|ch, _| ch.is_file_key_part());
            } else {
                let pos = tape.pos;
                return Err(DataError::IllegalCharacter { ch: tape[pos], pos });
            }
            if key.is_empty() {
                return Err(DataError::MissingValue { pos: tape.pos });
            }
            let key = str::from_utf8(key)?.to_string();

            // Parse assignment
            tape.consume(|ch, _| ch.is_file_ws());
            if tape.cur() != Some(b'=') {
                return Err(DataError::IllegalCharacter {
                    ch: tape.cur().unwrap_or(0),
                    pos: tape.pos,
                });
            }
            tape.adv(); // skip '='
            tape.consume(|ch, _| ch.is_file_ws());
            let val = self.parse_any(tape)?;

            map.insert(key, val);
        }
        Ok(DataValue::Object { tag, map })
    }

    fn parse_list(&self, tape: &mut Tape<'a, u8>) -> Result<DataValue, DataError> {
        let mut items = vec![];
        loop {
            tape.consume(|ch, _| ch.is_file_ws() || ch == b'\n');
            if tape.cur() == Some(b'}') {
                tape.adv();
                break;
            }
            if tape.cur().is_none() {
                return Err(DataError::MissingCloser {
                    open: b'{',
                    close: b'}',
                    open_pos: tape.pos,
                });
            }
            let val = self.parse_any(tape)?;
            items.push(val);
            tape.consume(|ch, _| ch.is_file_ws() || ch == b'\n');
            if tape.cur() == Some(b',') {
                tape.adv();
            } else if tape.cur() != Some(b'}') {
                return Err(DataError::IllegalCharacter {
                    ch: tape.cur().unwrap_or(0),
                    pos: tape.pos,
                });
            }
        }
        Ok(DataValue::List(items))
    }
}
