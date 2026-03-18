const IS_WS: u8 = 1 << 0; // 0000 0001
const IS_ROMAN: u8 = 1 << 1; // 0000 0010
const FLAG_BITS: u8 = 2;

// The table is exactly 256 bytes—one for every possible u8 value.
const CHAR_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];
    table[b' ' as usize] = IS_WS | (1 << FLAG_BITS);
    table[b'\t' as usize] = IS_WS | (4 << FLAG_BITS);
    table[b'\n' as usize] = IS_WS;
    table[b'\r' as usize] = IS_WS;
    let romans = b"ivxlcdmIVXLCDM";
    let mut i = 0;
    while i < romans.len() {
        table[romans[i] as usize] |= IS_ROMAN;
        i += 1;
    }
    table
};

pub trait CharExt {
    /// Returns true if the given character is a flanking
    /// whitespace character (space, tab, newline, or carriage return).
    ///
    /// This is used to determine whether certain characters
    /// (like `*` for bold/italic) should be treated as text or as formatting markers,
    /// based on their surrounding context.
    #[must_use]
    fn is_ws(&self) -> bool;

    /// Returns the length of the given whitespace character,
    /// where a tab counts as 4 spaces and space counts as 1.
    ///
    /// All other characters return a length of 0.
    #[must_use]
    fn ws_len(&self) -> u8;

    /// Returns true if the given character may be part of a Roman numeral.
    ///
    /// This is used to determine if a letter is part of a numeral in front of an ordered list item.
    #[must_use]
    fn is_roman(&self) -> bool;
}

impl CharExt for u8 {
    #[inline(always)]
    fn is_ws(&self) -> bool {
        (CHAR_TABLE[*self as usize] & IS_WS) != 0
    }

    #[inline(always)]
    fn ws_len(&self) -> u8 {
        // Shift the stored length value back down
        CHAR_TABLE[*self as usize] >> FLAG_BITS
    }

    #[inline(always)]
    fn is_roman(&self) -> bool {
        (CHAR_TABLE[*self as usize] & IS_ROMAN) != 0
    }
}
