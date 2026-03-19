//! Label formatting is made a non-issue due to extensions such as TabOut.

//auto de indent/indent on copypaste

//later: links like \a[1]{this} and
// \href{
//   1: google.com
// }
// virtual tokens
// auto-renumbering of list items by formatter
// mostly variable-length
// tokens do not need reflect text 1:1

use enum_ordinalize::Ordinalize;

/// The format in which a numbered list should be displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ordinalize)]
#[repr(u8)]
pub enum Numbering {
    Number,
    Lower,
    Upper,
    LowerNumeral,
    UpperNumeral,
    Continuation,
}

/// The type of an inline format marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ordinalize)]
#[repr(u8)]
pub enum InlineFormat {
    Bold,
    Italic,
    Strikethrough,
    Underline,
    Highlight,
}

impl InlineFormat {
    pub const BOLD_FLAG: u8 = 0b1;
    pub const ITALIC_FLAG: u8 = 0b10;
    pub const STRIKETHROUGH_FLAG: u8 = 0b100;
    pub const UNDERLINE_FLAG: u8 = 0b1000;
    pub const HIGHLIGHT_FLAG: u8 = 0b1_0000;
    const LENGTHS: [usize; 5] = [2, 1, 1, 1, 1];

    pub fn len(mask: u8) -> usize {
        if mask == Self::BOLD_FLAG | Self::ITALIC_FLAG {
            return 3;
        }
        Self::LENGTHS[mask.ilog2() as usize]
    }

    pub fn from_flag(flag: u8) -> Self {
        Self::VARIANTS[flag.ilog2() as usize]
    }
}

/// These include boundary markers (see comments).
///
/// Even if two tokens share the same structure and general purpose,
/// if their listener logic differs, then it is okay to differentiate them.
///
/// each corresponds to a listener logic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType<'a> {
    Plaintext,
    Newline,
    Literal { ch: u8 },
    LinkBody { href: &'a [u8] },       // ]( )
    LinkAliasBody { alias: &'a [u8] }, // ][ ]
    LinkMarker,
    EmbedMarker,
    MacroHandle { name: &'a [u8] }, // \
    MacroArgs { body: &'a [u8] },   // [ ]
    MacroBody { body: &'a [u8] },   // { }
    Heading { depth: u8 },
    InlineCode { body: &'a [u8] },    // ` `
    InlineRawCode { body: &'a [u8] }, // `` ``
    InlineMath { body: &'a [u8] },    // $ $
    CodeBlock { body: &'a [u8], lang: &'a [u8] },
    MathBlock { body: &'a [u8] },
    InlineFormat { ty: InlineFormat },
    Checkbox { depth: u8, filled: bool },
    ListItem { depth: u8 },
    NumberedItem { depth: u8, ty: Numbering },
}

impl<'a> TokenType<'a> {
    pub const HEADING_MAX: usize = 6;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'a> {
    pub ty: TokenType<'a>,
    pub start: usize,
    pub end: usize, // exclusive
}

impl<'a> Token<'a> {
    pub fn new(ty: TokenType<'a>, start: usize, end: usize) -> Self {
        Self { ty, start, end }
    }

    /// Guaranteed to be nonzero.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.end - self.start
    }
}
