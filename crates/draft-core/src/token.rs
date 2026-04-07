//! Label formatting is made a non-issue due to extensions such as TabOut.

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

impl Numbering {
    pub fn from_marker(marker: u8) -> Option<Self> {
        match marker {
            b'd' => Some(Numbering::Number),
            b'a' => Some(Numbering::Lower),
            b'A' => Some(Numbering::Upper),
            b'r' => Some(Numbering::LowerNumeral),
            b'R' => Some(Numbering::UpperNumeral),
            _ => None
        }
    }
}

/// The type of inline format marker.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CheckboxType {
    Filled, Empty, Interactable
}

impl CheckboxType {
    /// Returns the checkbox type according to the 
    pub fn from_marker(marker: u8) -> Option<Self> {
        match marker {
            b'x' => Some(CheckboxType::Filled),
            b'o' => Some(CheckboxType::Empty),
            b'?' => Some(CheckboxType::Interactable),
            _ => None
        }
    }
}

/// The class and payload of a token.
///
/// Tokens are categorized based on their unique function and listener logic.
/// 
/// Tokens containing each respective type include boundary markers
/// in range they represent (see comments).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType<'a> {
    Plaintext,// 
    Newline,
    HorizontalRule, // doubles as row divider, if enabled
    LinkMarker,//
    EmbedMarker,//
    LineQuoteMarker,
    BlockQuoteOpen,
    BlockQuoteClose,
    Literal { ch: u8 }, // preceded by `\`          //
    LinkBody { href: &'a [u8] },       // ]( )      //
    LinkAliasBody { alias: &'a [u8] }, // ][ ]      //
    MacroHandle { name: &'a [u8] },    // \      //
    MacroArgs { body: &'a [u8] },      // [ ]
    MacroBody { body: &'a [u8] },      // { }
    Heading { depth: u8 },
    InlineCode { body: &'a [u8] },    // ` `      //
    InlineRawCode { body: &'a [u8] }, // `` ``      //
    InlineMath { body: &'a [u8] },    // $ $      //
    CodeBlock { body: &'a [u8], lang: &'a [u8] },
    MathBlock { body: &'a [u8] },
    InlineFormat { ty: InlineFormat },      //
    Checkbox { depth: u8, ty: CheckboxType },
    ListItem { depth: u8 },
    NumberedItem { depth: u8, ty: Numbering },

    //todo works for citations via interpolation (`{paul}` => `[paul]=cite.{}`)
    AssignmentMarker { alias: &'a [u8] },    // [<key>]=<value>

    Eof,    // necessary to find bound for trailing plaintext
}

impl TokenType<'_> {
    pub const HEADING_MAX: usize = 6;
}

/// Represents a range of meaningful content in a markup file.
/// 
/// The end index is exclusive. 
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'a> {
    pub ty: TokenType<'a>,
    pub start: usize,
    pub end: usize,
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
