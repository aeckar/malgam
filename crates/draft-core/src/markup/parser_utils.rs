use derive_more::derive::Deref;

use crate::{
    markup::{
        lex::{ListItemKind, ListItemPos, Token, TokenKind, TokenSpan},
        parse::NodeMetadata as meta,
    },
    tape::Tape,
};

pub type TokenStream<'a> = Tape<'a, TokenSpan<'a>>;
pub type Result<'a> = Option<(AstNode<'a>, TokenStream<'a>)>;
pub type Handler<'a> = fn(TokenStream<'a>) -> Option<(AstNode<'a>, TokenStream<'a>)>;

/// A token or parser rule that can be matched to some slice of the
/// list of tokens produced after lexing.
pub trait SymbolKind {
    fn as_token_kind(self) -> Option<TokenKind>;
    fn as_rule_kind(self) -> Option<RuleKind>;
}

/// Rule identifiers, decoupled from rule matching logic to promote extensibility.
///
/// The suffix *-Kind* is used instead of *-Id* to avoid confusion with unique serial numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKind {
    Markup,
    TopLevelElement,
    Heading,
    Paragraph,
    Line,
    LineElement,
    Format,
    Link,
    Embed,
    List,
    LinkTarget,
    LineQuote,
    BlockQuote,
    ListItem,
    Macro,

    None,
}

impl SymbolKind for RuleKind {
    fn as_rule_kind(self) -> Option<RuleKind> {
        Some(self)
    }

    fn as_token_kind(self) -> Option<TokenKind> {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind<'a> {
    Rule(RuleKind),
    Token(Token<'a>),
}

impl<'a> NodeKind<'a> {
    #[inline]
    pub const fn token(self) -> Option<Token<'a>> {
        match self {
            Self::Token(token) => Some(token),
            _ => None,
        }
    }
}

impl<'a> SymbolKind for NodeKind<'a> {
    fn as_token_kind(self) -> Option<TokenKind> {
        match self {
            Self::Token(_) => None,
            Self::Rule(_) => None,
        }
    }

    fn as_rule_kind(self) -> Option<RuleKind> {
        match self {
            Self::Rule(rule) => Some(rule),
            Self::Token(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeMetadata {
    Choice(u8),
    IsPresent(bool),
    ListItem { kind: ListItemKind, pos: ListItemPos },
    None,
}

/// Dereferences to its `children` vector.
///
/// `end` is exclusive.
#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub struct AstNode<'a> {
    #[deref]
    pub children: Vec<AstNode<'a>>,

    pub meta: NodeMetadata,
    pub parent: Option<RuleKind>,
    pub start: usize,
    pub end: usize,
    pub kind: NodeKind<'a>,
}

impl<'a> AstNode<'a> {
    /// Returns a rule node that may be either a leaf or a branch.
    pub fn new(
        rule: RuleKind,
        mut children: Vec<AstNode<'a>>,
        pos: usize,
        meta: NodeMetadata,
    ) -> Self {
        if children.is_empty() {
            return Self {
                start: pos,
                end: pos,
                parent: None,
                children,
                meta,
                kind: NodeKind::Rule(rule),
            };
        }
        for child in children.iter_mut() {
            child.parent = Some(rule)
        }
        Self {
            start: children[0].start,
            end: children.last().unwrap().end,
            parent: None,
            children,
            meta,
            kind: NodeKind::Rule(rule),
        }
    }

    /// Returns a rule branch node.
    ///
    /// Panics if `children` is empty.
    pub fn branch(rule: RuleKind, mut children: Vec<AstNode<'a>>, meta: NodeMetadata) -> Self {
        if children.is_empty() {
            panic!("Missing children for rule {rule:?}")
        }
        for child in children.iter_mut() {
            child.parent = Some(rule)
        }
        Self {
            start: children[0].start,
            end: children.last().unwrap().end,
            parent: None,
            children,
            meta,
            kind: NodeKind::Rule(rule),
        }
    }

    /// Returns a token leaf node using the next token span in the tape.
    ///
    /// Panics if `tape` is exhausted.
    #[inline]
    pub fn token(span: TokenSpan<'a>) -> Self {
        Self {
            start: span.start,
            end: span.end,
            parent: None,
            children: vec![],
            meta: meta::None,
            kind: NodeKind::Token(span.token),
        }
    }

    /// Returns a token leaf node using the next token span in the tape,
    /// incrementing `tape.pos` on success.
    ///
    /// Panics if the tape is exhausted.
    pub fn try_token(token: TokenKind, tape: &mut TokenStream<'a>) -> Option<Self> {
        if tape.peek().is_none_or(|span| span.token.kind() != token) {
            return None;
        }
        let span = tape.next().unwrap();
        Some(Self {
            start: span.start,
            end: span.end,
            parent: None,
            children: vec![],
            meta: meta::None,
            kind: NodeKind::Token(span.token),
        })
    }

    #[inline]
    pub fn is_leaf(&self) -> bool {
        matches!(self.kind, NodeKind::Token(_))
    }

    #[inline]
    pub fn is_branch(&self) -> bool {
        matches!(self.kind, NodeKind::Rule(_))
    }
}
