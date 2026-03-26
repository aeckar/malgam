const IS_WS: u8 = 1 << 0; // 0000_0001
const FLAG_BITS: u8 = 1;

/// Exactly 256 bytes—one for every possible u8 value.
const CHAR_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];
    table[b' ' as usize] = IS_WS | (1 << FLAG_BITS);
    table[b'\t' as usize] = IS_WS | (4 << FLAG_BITS);
    table[b'\n' as usize] = IS_WS;
    table[b'\r' as usize] = IS_WS | (1 << FLAG_BITS);
    table
};

pub trait CharExt {
    /// Returns true if the given character is a flanking
    /// whitespace character (space, tab, newline, or carriage return).
    ///
    /// This is used to determine whether certain characters
    /// (like `*` for bold/italic) should be treated as text or as formatting markers,
    /// based on their surrounding context.
    /// 
    /// Recognition of these whitespace characters extends to HGON also.
    #[must_use]
    fn is_flank_ws(&self) -> bool;

    /// Returns the length of the given flanking whitespace character,
    /// where a tab counts as 4 spaces and space counts as 1.
    ///
    /// All other characters return a length of 0.
    #[must_use]
    fn flank_ws_len(&self) -> u8;
}

impl CharExt for u8 {
    #[inline(always)]
    fn is_flank_ws(&self) -> bool {
        (CHAR_TABLE[*self as usize] & IS_WS) != 0
    }

    #[inline(always)]
    fn flank_ws_len(&self) -> u8 {
        // Shift the stored length value back down
        CHAR_TABLE[*self as usize] >> FLAG_BITS
    }
}

pub trait SliceExt {
    /// Returns a subslice with leading and trailing white space removed,
    /// according to the compiler.
    fn trim_ws(&self) -> Self;
}

impl SliceExt for &[u8] {
    #[inline(always)]
    fn trim_ws(&self) -> Self {
        let mut bytes = *self;
        while let [first, rest @ ..] = bytes {
            // peel off front
            if first.is_flank_ws() {
                bytes = rest;
            } else {
                break;
            }
        }
        while let [rest @ .., last] = bytes {
            // peel off back
            if last.is_flank_ws() {
                bytes = rest;
            } else {
                break;
            }
        }
        bytes
    }
}
