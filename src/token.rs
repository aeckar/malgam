//! Label formatting is made a non-issue due to extensions such as TabOut.

//auto de indent/indent on copypaste

//later: links like \a[1]{this} and
// \href{
//   1: google.com
// }
// todo includes virtual tokens
// auto-renumbering of list items by formatter
// mostly variable-length
// tokens do not need reflect text 1:1

/// The format in which a numbered list should be displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NumberingType {
    Number,
    Lower,
    Upper,
    LowerNumeral,
    UpperNumeral,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    Literal { ch: u8 },
    Link { embed: bool, alt: String, href: String },
    LinkAlias { embed: bool, alt: String, href: String, alias: String },
    MacroHandle { name: String },
    MacroArgs { body: String },
    MacroBody { body: String },
    Heading { depth: u8 },
    InlineCode { body: String },    // includes ` `
    InlineRawCode { body: String }, // includes `` ``
    InlineMath { body: String },    // includes $ $
    Bold,
    Italic,
    Strikethrough,
    Underline,
    Highlight,
    Checkbox { depth: u8, filled: bool },
    ListItem { depth: u8 },
    NumberedItem { depth: u8, ty: NumberingType, pos: u8 },

    Brac // not actually emitted
}

impl TokenType {
    pub const HEADING_MAX: usize = 6;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub ty: TokenType,
    pub start: usize,
    pub end: usize, // exclusive
}

impl Token {
    pub fn new(ty: TokenType, start: usize, end: usize) -> Self {
        Self { ty, start, end }
    }
}
