use std::sync::OnceLock;

use bitflags::bitflags;
use strum::EnumDiscriminants;

use crate::markup::parse::{RuleKind, SymbolKind};

/// Unpacks a specific enum variant from a token, destructuring its fields into local variables.
///
/// This macro simplifies extracting data from `$crate::markup::lex::Token`. It performs
/// an immutable borrow of the token and uses a `let-else` statement to panic if
/// the variant does not match the expected type.
///
/// # Arguments
/// * `$instance` - An expression that provides access to the token (e.g., an AST node or wrapper).
/// * `$variant` - The specific `Token` variant name to match (e.g., `Identifier`).
/// * `{ $($field ... )* }` - A standard destructuring block. Supports field renaming
///   (`field: alias`) and the `..` rest pattern.
///
/// # Panics
/// Panics if the token is `None` (via `.unwrap()`) or if the token's variant
/// does not match `$variant`.
///
/// # Examples
/// ```
/// // Simple destructuring: creates local variables 'name' and 'span'
/// unpack_token!(node, Identifier { name, span });
///
/// // With renaming and rest pattern: creates variable 'val' from 'value'
/// unpack_token!(node, Literal { value: val, .. });
/// ```
#[macro_export]
macro_rules! unpack_token {
    ($instance:expr, $variant:ident { $($field:ident $(: $alias:ident)?),* $(, $(..)?)? }) => {
        let $crate::markup::lex::Token::$variant {
            $($field $(: $alias)?),* , ..
        } = $instance.kind.token().unwrap() else {
            panic!("Unpack failed: Expected {}", stringify!($variant));
        };
    };
}

static FORMAT_VARIANTS: OnceLock<Vec<InlineFormat>> = OnceLock::new();

/// The format in which a numbered list should be displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Numbering {
    Number,
    Lower,
    Upper,
    LowerNumeral,
    UpperNumeral,
}

impl Numbering {
    #[inline]
    pub const fn from_marker(marker: u8) -> Option<Self> {
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

bitflags::bitflags! {
    /// The type of inline format marker.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct InlineFormat: u8 {
        const BOLD = 0b0000_0001;
        const ITALIC = 0b0000_0010;
        const STRIKETHROUGH = 0b0000_0100;
        const UNDERLINE = 0b000_1000;
        const HIGHLIGHT = 0b0001_0000;

        const BOLD_ITALIC = Self::BOLD.bits() | Self::ITALIC.bits();
    }
}

impl InlineFormat {
    /// Returns the length of the character cluster that denotes the given flag or bitmask.
    #[inline]
    pub fn len(self) -> usize {
        if self == Self::BOLD_ITALIC {
            return 3;
        }
        if self == Self::BOLD {
            return 2;
        }
        return 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckboxType {
    Filled,
    Empty,
    Toggle,
}

impl CheckboxType {
    /// Returns the checkbox type according to the
    pub const fn from_marker(marker: u8) -> Option<Self> {
        match marker {
            b'x' => Some(CheckboxType::Filled),
            b'o' => Some(CheckboxType::Empty),
            b'?' => Some(CheckboxType::Toggle),
            _ => None,
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ListItemPos: u8 {
        const Any = 0b0000;
        const First = 0b0001;
        const Last = 0b0010;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListItemKind {
    Unordered,
    Continuation,
    Numbered(Numbering),
    Checkbox(CheckboxType),
}

impl ListItemKind {
    /// Returns true if both kinds of list items can reside within the same list.
    #[inline]
    pub fn is_sibling(self, other: Self) -> bool {
        if self == Self::Unordered {
            return other == Self::Unordered;
        }
        if matches!(self, Self::Numbered(_)) {
            return self == other;
        }
        debug_assert!(matches!(self, Self::Checkbox(_)));
        return matches!(other, Self::Checkbox(_));
    }

    /// Returns the open tag, or panics if this is a continuation.
    #[inline]
    pub const fn open_tag(self) -> &'static str {
        match self {
            Self::Unordered => "ul class='dt-Unordered'",
            Self::Numbered(ty) => match ty {
                Numbering::Number => "ol class='dt-numbering'",
                Numbering::Lower => "ol type='a' class='dt-numbering'",
                Numbering::Upper => "ol type='A' class='dt-numbering'",
                Numbering::LowerNumeral => "ol type='i' class='dt-numbering'",
                Numbering::UpperNumeral => "ol type='I' class='dt-numbering'",
            },
            Self::Checkbox(ty) => match ty {
                CheckboxType::Empty => "ol class='dt-checkbox--empty'",
                CheckboxType::Filled => "ol class='dt-checkbox--filled'",
                CheckboxType::Toggle => "ol class='det-checkbox--toggle'",
            },
            Self::Continuation => panic!("Cannot resolve open tag"),
        }
    }

    /// Returns the open tag, or panics if this is a continuation.
    #[inline]
    pub const fn close_tag(self) -> &'static str {
        match self {
            Self::Unordered => "ul",
            Self::Continuation => panic!("Cannot resolve close tag"),
            _ => "ol",
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
    InferredLink { href: &'a [u8] },
    LinkMarker,
    EmbedMarker,
    MacroHandle { name: &'a [u8] },   // \[ ]
    InlineCode { body: &'a [u8] },    // ` `
    InlineRawCode { body: &'a [u8] }, // `` ``
    InlineMath { body: &'a [u8] },    // $ $
    InlineFormat { ty: InlineFormat, twin_pos: usize },

    // Everything else
    Newline,
    HorizontalRule, // doubles as row divider, if enabled
    LineQuoteMarker,
    BlockQuoteOpen,
    BlockQuoteClose,
    MacroDeco { body: &'a [u8] },   // ( )
    MacroConfig { body: &'a [u8] }, // [ ]
    MacroBody { body: &'a [u8] },   // { }
    HeadingMarker { depth: u8 },
    CodeBlock { body: &'a [u8], lang: &'a [u8] },
    MathBlock { body: &'a [u8] },
    ListItemMarker { indent: u8, kind: ListItemKind },
    Assignment { key: &'a [u8], value_idx: usize },
    Eof, // necessary to find bound for trailing plaintext; pruned before parsing
}

impl Token<'_> {
    pub const HEADING_MAX: usize = 6;

    #[inline]
    pub const fn is_content(self) -> bool {
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

    #[inline]
    pub fn kind(&self) -> TokenKind {
        TokenKind::from(self)
    }
}

impl SymbolKind for TokenKind {
    #[inline]
    fn as_rule_kind(self) -> Option<RuleKind> {
        None
    }

    #[inline]
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
    #[inline]
    pub const fn new(token: Token<'a>, start: usize, end: usize) -> Self {
        Self { token, start, end }
    }

    /// Guaranteed to be nonzero.
    #[inline]
    pub const fn len(&self) -> usize {
        self.end - self.start
    }

    /// Marks this span as plaintext.
    #[inline]
    pub const fn bind_plain(&mut self) {
        self.token = Token::Plaintext;
    }
}
