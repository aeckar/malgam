use bitflags::Flags;

bitflags::bitflags! {
    #[derive(Copy, Clone)]
    struct CharType: u8 {
        const IS_KEY_PART = 0b0001;
        const IS_KEY_START = 0b0010;
        const IS_FILE_WS  = 0b0100;
        // remaining bits are for length
    }
}

impl CharType {
    #[inline]
    const fn with_len(self, len: u8) -> u8 {
        self.bits() | (len << Self::FLAGS.len())
    }
}

/// One byte for every possible `u8` value.
const CHAR_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];
    table[b' ' as usize] = CharType::IS_FILE_WS.with_len(1);
    table[b'\t' as usize] = CharType::IS_FILE_WS.with_len(4);
    table[b'\r' as usize] = CharType::IS_FILE_WS.with_len(1);

    // Get starts
    let starts = concat!(
        "abcdefghijklmnopqrstuvwxyz",
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        "$",
    )
    .as_bytes();
    let mut i = 0;
    while i < starts.len() {
        table[starts[i] as usize] = CharType::IS_KEY_START.bits();
        i += 1;
    }

    // Get parts
    let parts = concat!(
        "abcdefghijklmnopqrstuvwxyz",
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        "0123456789",
        "-_.$",
    )
    .as_bytes();
    let mut i = 0;
    while i < parts.len() {
        table[parts[i] as usize] = CharType::IS_KEY_PART.bits();
        i += 1;
    }

    table
};

pub trait CharExt {
    /// Returns true if this is a flanking
    /// whitespace character (space, tab, or carriage return).
    ///
    /// This is used to determine whether certain characters
    /// (like `*` for bold/italic) should be treated as text or as formatting markers,
    /// based on their surrounding context.
    ///
    /// Recognition of these whitespace characters extends to object notation also.
    #[must_use]
    fn is_file_ws(self) -> bool;

    /// Returns the length of the given flanking whitespace character,
    /// where a tab counts as 4 spaces and space counts as 1.
    ///
    /// All other characters return a length of 0.
    #[must_use]
    fn file_ws_len(self) -> u8;

    /// Returns true if this character may be part of an unescaped (without `[]`) key
    /// in object notation.
    ///
    /// Keys must start with a letter or dollar sign (signalling meta-properties).
    ///
    /// Keys are case-insensitive.
    fn is_file_key_start(self) -> bool;

    /// Returns true if this character may be part of an unescaped (without `[]`) key
    /// in object notation.
    ///
    /// Letters, digits, dashes, underscores, dots, and dollar signs are accepted.
    /// Kebab case is used, with dots used to denote scope and dollar signs
    /// used to denote special keys.
    ///
    /// Underscores are given as alternatives to dashes as a way to keep parity with CSS
    /// if an object is used for styling, and are treated as equivalent during parsing.
    ///
    /// Keys are case-insensitive.
    fn is_file_key_part(self) -> bool;
}

impl CharExt for u8 {
    #[inline]
    fn is_file_key_part(self) -> bool {
        (CHAR_TABLE[self as usize] & CharType::IS_KEY_PART.bits()) != 0
    }

    #[inline]
    fn is_file_ws(self) -> bool {
        (CHAR_TABLE[self as usize] & CharType::IS_FILE_WS.bits()) != 0
    }

    #[inline]
    fn is_file_key_start(self) -> bool {
        (CHAR_TABLE[self as usize] & CharType::IS_KEY_START.bits()) != 0
    }

    #[inline]
    fn file_ws_len(self) -> u8 {
        // shift the stored length value back down
        CHAR_TABLE[self as usize] >> CharType::FLAGS.len()
    }
}

pub trait SliceExt<'a> {
    /// Returns a subslice with leading and trailing flanking white space removed.
    fn trim_file_ws(self) -> Self;

    /// Returns the top-level domain (TLD) of the link, which is assumed to be valid.
    fn tld(self) -> Self;
}

impl<'a> SliceExt<'a> for &'a [u8] {
    fn trim_file_ws(mut self) -> Self {
        while let [first, rest @ ..] = self {
            // peel off front
            if first.is_file_ws() {
                self = rest;
            } else {
                break;
            }
        }
        while let [rest @ .., last] = self {
            // peel off back
            if last.is_file_ws() {
                self = rest;
            } else {
                break;
            }
        }
        self
    }

    fn tld(self) -> Self {
        let mut dot_idx = 0;
        for (idx, &c) in self.iter().enumerate() {
            if c == b'/' {
                if idx == dot_idx + 1 {
                    panic!("Invalid URL");
                }
                return &self[dot_idx + 1..idx];
            }
            if c == b'.' {
                dot_idx = idx;
            }
        }
        panic!("Invalid URL");
    }
}
