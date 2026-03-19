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
    Continuation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType<'a> {
    Literal { ch: u8 },
    Link { embed: bool, alt: &'a [u8], href: &'a [u8] },
    LinkAlias { embed: bool, alt: &'a [u8], href: &'a [u8], alias: &'a [u8] },
    MacroHandle { name: &'a [u8] },
    MacroArgs { body: &'a [u8] },
    MacroBody { body: &'a [u8] },
    Heading { depth: u8 },
    InlineCode { body: &'a [u8] },    // includes ` `
    InlineRawCode { body: &'a [u8] }, // includes `` ``
    InlineMath { body: &'a [u8] },    // includes $ $
    CodeBlock { body: &'a [u8], lang: &'a [u8] },
    MathBlock {body: &'a [u8]},
    Bold,
    Italic,
    Strikethrough,
    Underline,
    Highlight,
    Checkbox { depth: u8, filled: bool },
    ListItem { depth: u8 },
    NumberedItem { depth: u8, ty: NumberingType },
}

impl TokenType {
    pub const HEADING_MAX: usize = 6;
    pub const FLANK: [TokenType; 5] = [
            TokenType::Bold,
    TokenType::Italic,
    TokenType::Strikethrough,
    TokenType::Underline,
    TokenType::Highlight,
    ];
}

pub struct FlankType;

impl FlankType {
    pub const BOLD: u8 = 0b1;
    pub const ITALIC: u8 = 0b10;
    pub const STRIKETHROUGH: u8 = 0b100;
    pub const UNDERLINE: u8 = 0b1000;
    pub const HIGHLIGHT: u8 = 0b1_0000;
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
