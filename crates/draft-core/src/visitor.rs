use crate::ast::{AstNode, BlockNode, InlineNode, ListNode};

/// A visitor trait for traversing and processing AST (Abstract Syntax Tree) nodes.
///
/// This trait defines a set of methods for visiting different types of nodes in the AST.
/// Implementors can use mutable references to maintain state during traversal.
///
/// # Note
///
/// Passing mutable references to visitors is permissible as there is no intention to enable
/// parallelization at this time.
///
/// # Methods
///
/// The trait provides specialized visitor methods for different node types:
/// - `visit_ast_node`: Visit a generic AST node
/// - `visit_block_node`: Visit a block-level node
/// - `visit_list_node`: Visit a list node
/// - `visit_inline_node`: Visit an inline node
///
/// # Generic Parameter
///
/// Each method is generic over return type `Self::Output`, allowing visitors to return different types
/// of results depending on the implementation and context.
pub trait AstVisitor<'a> {
    type Output;

    fn visit_ast_node(&mut self, node: &AstNode<'a>) -> Self::Output;
    fn visit_block_node(&mut self, node: &BlockNode<'a>) -> Self::Output;
    fn visit_list_node(&mut self, node: &ListNode<'a>) -> Self::Output;
    fn visit_inline_node(&mut self, node: &InlineNode<'a>) -> Self::Output;
}

pub struct AstToHtml {}

// todo integrate arena alloc
impl<'a> AstVisitor<'a> for AstToHtml {
    type Output = Vec<String>;

    fn visit_ast_node(&mut self, node: &AstNode<'a>) -> Self::Output {
        match node {
            AstNode::Block(block) => block,
            AstNode::Inline(inline) => inline,
        }
    }

    fn visit_block_node(&mut self, node: &BlockNode<'a>) -> Self::Output {
        fn visit_block_node(&mut self, node: &BlockNode<'a>) -> Self::Output {
            match node {
                BlockNode::Paragraph(content) => todo!(),
                BlockNode::Heading(level, content) => todo!(),
                BlockNode::BlockQuote(children) => todo!(),
                BlockNode::List(list) => self.visit_list_node(list),
                BlockNode::CodeBlock(info, code) => todo!(),
                BlockNode::ThematicBreak => todo!(),
                BlockNode::Html(html) => todo!(),
            }
        }
    }

    fn visit_list_node(&mut self, node: &ListNode<'a>) -> Self::Output {
        match node {
            ListNode::Unordered(items) => todo!(),
            ListNode::Ordered(start, items) => todo!(),
        }
    }

    fn visit_inline_node(&mut self, node: &InlineNode<'a>) -> Self::Output {
        match node {
            InlineNode::Text(text) => todo!(),
            InlineNode::SoftBreak => todo!(),
            InlineNode::HardBreak => todo!(),
            InlineNode::Code(code) => todo!(),
            InlineNode::Html(html) => todo!(),
            InlineNode::Emphasis(children) => todo!(),
            InlineNode::Strong(children) => todo!(),
            InlineNode::Link(url, title, children) => todo!(),
            InlineNode::Image(url, title, children) => todo!(),
            _ => todo!(),
        }
    }
}

pub struct AstToMarkdown {}

impl<'a> AstVisitor<'a> for AstToMarkdown {
    type Output = Vec<String>;

    fn visit_ast_node(&mut self, node: &AstNode<'a>) -> Self::Output {
        match node {
            AstNode::Block(block) => block,
            AstNode::Inline(inline) => inline,
        }
    }

    fn visit_block_node(&mut self, node: &BlockNode<'a>) -> Self::Output {
        fn visit_block_node(&mut self, node: &BlockNode<'a>) -> Self::Output {
            match node {
                BlockNode::Paragraph(content) => todo!(),
                BlockNode::Heading(level, content) => todo!(),
                BlockNode::BlockQuote(children) => todo!(),
                BlockNode::List(list) => self.visit_list_node(list),
                BlockNode::CodeBlock(info, code) => todo!(),
                BlockNode::ThematicBreak => todo!(),
                BlockNode::Html(html) => todo!(),
            }
        }
    }

    fn visit_list_node(&mut self, node: &ListNode<'a>) -> Self::Output {
        match node {
            ListNode::Unordered(items) => todo!(),
            ListNode::Ordered(start, items) => todo!(),
        }
    }

    fn visit_inline_node(&mut self, node: &InlineNode<'a>) -> Self::Output {
        match node {
            InlineNode::Text(text) => todo!(),
            InlineNode::SoftBreak => todo!(),
            InlineNode::HardBreak => todo!(),
            InlineNode::Code(code) => todo!(),
            InlineNode::Html(html) => todo!(),
            InlineNode::Emphasis(children) => todo!(),
            InlineNode::Strong(children) => todo!(),
            InlineNode::Link(url, title, children) => todo!(),
            InlineNode::Image(url, title, children) => todo!(),
            _ => todo!(),
        }
    }
}
z