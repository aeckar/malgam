use simdutf8::basic::Utf8Error;
use thiserror::Error;

use crate::prelude::*;
use crate::markup::{lexer::MarkupLexerError, vocab::Token};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstNode<'a> {
    pub token: Option<Token<'a>>,
    pub parent: Option<&'a AstNode<'a>>,
    pub children: Vec<AstNode<'a>>,
}

impl<'a> AstNode<'a> {
    pub fn root() -> Self {
        Self {
            token: None,
            parent: None,
            children: vec![],
        }
    }

    pub fn new(token: Token<'a>, parent: &'a AstNode<'a>) -> Self {
        Self {
            token: Some(token),
            parent: Some(parent),
            children: vec![],
        }
    }
}


#[derive(Error, Debug)]
pub enum MarkupParserError {
    #[error("Invalid UTF-8")]
    InvalidUtf8(#[from] Utf8Error),
}

/// Assembles the AST according to the following grammar:
/// ```ebnf
/// topLevelElement := HorizontalRule
///     | CodeBlock
///     | MathBlock
///     | paragraph
///     | list
///     | heading
///     | lineQuote
///     | blockQuote
/// heading := Heading
///     & line
///     & Newline
/// paragraph := Plaintext | Literal | link
///
/// line := lineElement+
///     & Newline
/// lineElement := Plaintext
///     | InlineCode
///     | InlineMath
///     | InlineRawCode
///     | Literal
///     | format
///     | link
///     | embed
///
/// format := InlineFormat plaintext InlineFormat
/// link := LinkMarker & linkTarget
/// embed := EmbedMarker & linkTarget
/// linkTarget := LinkBody | LinkAliasBody
///
/// lineQuote := LineQuoteMarker & line
/// blockQuote := BlockQuoteOpen
///     & (line | Newline)
///     & topLevelElement+
///     & BlockQuoteClose
///
/// list := orderedList | numberedList | checklist
/// orderedList := (ListItemMarker & line)+
/// numberedList := (NumberedItemMarker & line)+
/// checklist := (Checkbox & line)+
///
/// macro := MacroHandle
///     & MacroArgs?
///     & MacroBody*
/// ```
///
/// For constructing new grammars, the following protocol usually suffices:
/// 1. Make rules for tokens that easily combine
/// 2. Combine rules into abstract concepts
/// 3. Seperate elements by creating rules for top-level and inline nodes
struct MarkupParser<'a> {
    // All tokens in the markup file.
    tokens: &'a [Token<'a>],
}

impl<'a> Compile for MarkupParser<'a> {
    type Output = Result<AstNode<'a>, MarkupLexerError>;

    fn compile(self) -> Self::Output {
        self.top_level_element(Tape::new(self.tokens))
    }
}

impl<'a> MarkupParser<'a> {
    fn top_level_element(
        &self,
        mut tape: Tape<'a, Token>,
    ) -> Result<AstNode<'a>, MarkupLexerError> {
    }
}
