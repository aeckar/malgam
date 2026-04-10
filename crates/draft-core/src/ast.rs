use crate::token::Token;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstNode<'a> {
    token: Token<'a>,
    parent: Option<&'a AstNode<'a>>,
    children: Vec<AstNode<'a>>,
}