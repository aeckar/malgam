use std::str::Utf8Error;

const IS_FILE_WS: u8 = 1 << 0; // 0000_0001
const IS_KEY_PART: u8 = 1 << 1;    // 0000_0010
const FLAG_BITS: u8 = 2;

/// Exactly 256 bytes—one for every possible u8 value.
const CHAR_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];
    table[b' ' as usize] = IS_FILE_WS | (1 << FLAG_BITS);
    table[b'\t' as usize] = IS_FILE_WS | (4 << FLAG_BITS);
    table[b'\r' as usize] = IS_FILE_WS | (1 << FLAG_BITS);
    
    let bytes = concat!(
        "abcdefghijklmnopqrstuvwxyz",
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        "0123456789",
        "-_.$"
    ).as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        table[bytes[i] as usize] = IS_KEY_PART;
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
    fn is_file_ws(&self) -> bool;

    /// Returns the length of the given flanking whitespace character,
    /// where a tab counts as 4 spaces and space counts as 1.
    ///
    /// All other characters return a length of 0.
    #[must_use]
    fn file_ws_len(&self) -> u8;

    /// Returns true if this character may be part of an unescaped (without `""`) key
    /// in object notation.
    /// 
    /// Letters, digits, dashes, underscores, dots, and dollar signs are accepted.
    fn is_file_key_part(&self) -> bool;
}

impl CharExt for u8 {
    #[inline(always)]
    fn is_file_key_part(&self) -> bool {
        (CHAR_TABLE[*self as usize] & IS_KEY_PART) != 0
    }

    #[inline(always)]
    fn is_file_ws(&self) -> bool {
        (CHAR_TABLE[*self as usize] & IS_FILE_WS) != 0
    }

    #[inline(always)]
    fn file_ws_len(&self) -> u8 {
        // Shift the stored length value back down
        CHAR_TABLE[*self as usize] >> FLAG_BITS
    }
}

pub trait SliceExt {
    /// Returns a subslice with leading and trailing flanking white space removed.
    fn trim_file_ws(&self) -> Self;

    /// Attempts to convert this slice to a UTF-8 string with the same contents.
    fn to_utf8(&self) -> Result<String, Utf8Error>;
}

impl SliceExt for &[u8] {
    #[inline(always)]
    fn trim_file_ws(&self) -> Self {
        let mut bytes = *self;
        while let [first, rest @ ..] = bytes {
            // peel off front
            if first.is_file_ws() {
                bytes = rest;
            } else {
                break;
            }
        }
        while let [rest @ .., last] = bytes {
            // peel off back
            if last.is_file_ws() {
                bytes = rest;
            } else {
                break;
            }
        }
        bytes
    }

    fn to_utf8(&self) -> Result<String, Utf8Error> {
        String::from_utf8(self.to_vec()).map_err(|e| e.utf8_error())
    }   
}