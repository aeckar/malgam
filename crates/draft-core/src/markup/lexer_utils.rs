use std::sync::OnceLock;

use strum::{EnumDiscriminants, EnumIter, IntoEnumIterator};

use crate::markup::{parse::RuleKind, parser_utils::Symbol};

static INLINE_FMT_VARIANTS: OnceLock<Vec<InlineFormat>> = OnceLock::new();

/// The format in which a numbered list should be displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            _ => None,
        }
    }
}

/// The type of inline format marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
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

    /// Returns the length of the character cluster that denotes the given flag or bitmask.
    pub fn len(mask: u8) -> usize {
        if mask == Self::BOLD_FLAG | Self::ITALIC_FLAG {
            return 3;
        }
        Self::LENGTHS[mask.ilog2() as usize]
    }

    /// Panics if a bitmask or invalid flag is given.
    pub fn from_flag(flag: u8) -> Self {
        Self::variants()[flag.ilog2() as usize]
    }

    fn variants() -> &'static Vec<InlineFormat> {
        INLINE_FMT_VARIANTS.get_or_init(|| InlineFormat::iter().collect())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CheckboxType {
    Filled,
    Empty,
    Interactable,
}

impl CheckboxType {
    /// Returns the checkbox type according to the
    pub fn from_marker(marker: u8) -> Option<Self> {
        match marker {
            b'x' => Some(CheckboxType::Filled),
            b'o' => Some(CheckboxType::Empty),
            b'?' => Some(CheckboxType::Interactable),
            _ => None,
        }
    }
}

/// The class and payload of a token.
///
/// Tokens are categorized based on their unique function and listener logic.
///
/// Tokens containing each respective type include boundary markers
/// in range they represent (see comments).
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumDiscriminants)]
#[strum_discriminants(name(TokenKind))]
pub enum Token<'a> {
    // Content
    Plaintext,
    Literal { ch: u8 },                // preceded by `\`
    LinkBody { href: &'a [u8] },       // ]( )
    LinkAliasBody { alias: &'a [u8] }, // ][ ]
    LinkMarker,
    EmbedMarker,
    MacroHandle { name: &'a [u8] },   // \
    InlineCode { body: &'a [u8] },    // ` `
    InlineRawCode { body: &'a [u8] }, // `` ``
    InlineMath { body: &'a [u8] },    // $ $
    InlineFormat { ty: InlineFormat },

    // Everything else
    Newline,
    HorizontalRule, // doubles as row divider, if enabled
    LineQuoteMarker,
    BlockQuoteOpen,
    BlockQuoteClose,
    MacroArgs { body: &'a [u8] }, // [ ]
    MacroBody { body: &'a [u8] }, // { }
    HeadingMarker { depth: u8 },
    CodeBlock { body: &'a [u8], lang: &'a [u8] },
    MathBlock { body: &'a [u8] },
    Checkbox { depth: u8, ty: CheckboxType },
    ListItemMarker { depth: u8 },
    NumberedItemMarker { depth: u8, ty: Numbering },
    AssignmentMarker { alias: &'a [u8] }, // [<key>]=<value>//todo works for citations via interpolation (`{paul}` => `[paul]=cite.{}`)
    Eof, // necessary to find bound for trailing plaintext; pruned before parsing
}

impl Token<'_> {
    pub const HEADING_MAX: usize = 6;

    pub fn is_content(self) -> bool {
        matches!(
            self,
            Self::Plaintext
                | Self::Literal { .. }
                | Self::LinkBody { .. }
                | Self::LinkAliasBody { .. }
                | Self::LinkMarker
                | Self::EmbedMarker
                | Self::MacroHandle { .. }
                | Self::InlineCode { .. }
                | Self::InlineRawCode { .. }
                | Self::InlineMath { .. }
                | Self::InlineFormat { .. }
        )
    }

    pub fn kind(&self) -> TokenKind {
        TokenKind::from(self)
    }
}

impl Symbol for TokenKind {
    fn as_rule_kind(self) -> Option<RuleKind> {
        None
    }
    fn as_token_kind(self) -> Option<TokenKind> {
        Some(self)
    }
}

/// Represents a range of meaningful content in a markup file.
///
/// The end index is exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenSpan<'a> {
    pub token: Token<'a>,
    pub start: usize,
    pub end: usize,
}

impl<'a> TokenSpan<'a> {
    pub fn new(token: Token<'a>, start: usize, end: usize) -> Self {
        Self { token, start, end }
    }

    /// Guaranteed to be nonzero.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Marks this span as plaintext.
    pub fn bind_plain(&mut self) {
        self.token = Token::Plaintext;
    }
}
