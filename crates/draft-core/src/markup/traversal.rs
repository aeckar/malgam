use pastey::paste;

use crate::markup::parser_utils::AstNode;

macro_rules! visits {
    ($name:ident $(,)?) => {
        paste! {
            fn [< visit_ $name >](&mut self, node: &AstNode<'a>) -> Self::Output;
        }
    };
}

macro_rules! visitor {
    ($name:ident, $body:expr, $(,)?) => {
        paste! {
            #[inline(always)]
            fn [< visit_ $name >](&mut self, node: &AstNode<'a>) -> Self::Output {
                body()
            };
        }
    };
}

macro_rules! noop {
    ($name:ident $(,)?) => {
        paste! {
            fn [< visit_ $name >](&mut self, node: &AstNode<'a>) -> Self::Output {};
        }
    };
}

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

    // Rules
    visits!(markup);
    visits!(top_level_element);
    visits!(heading);
    visits!(line);
    visits!(line_element);
    visits!(format);
    visits!(link);
    visits!(embed);
    visits!(link_target);
    visits!(list);
    visits!(unordered_list);
    visits!(numbered_list);
    visits!(checklist);
    visits!(line_quote);
    visits!(block_quote);
    visits!(macro_rule);

    // Tokens
    visits!(plaintext);
    visits!(literal);
    visits!(link_body);
    visits!(link_alias_body);
    visits!(link_marker);
    visits!(embed_marker);
    visits!(macro_handle);
    visits!(inline_code);
    visits!(inline_raw_code);
    visits!(inline_math);
    visits!(inline_format);
    visits!(newline);
    visits!(horizontal_rule);
    visits!(line_quote_marker);
    visits!(block_quote_open);
    visits!(block_quote_close);
    visits!(macro_args);
    visits!(macro_body);
    visits!(heading_marker);
    visits!(code_block);
    visits!(math_block);
    visits!(checkbox);
    visits!(list_item_marker);
    visits!(numbered_item_marker);
    visits!(assignment_marker);
}

pub struct AstToHtml {}

// todo integrate arena alloc
impl<'a> AstVisitor<'a> for AstToHtml {
    type Output = Vec<String>;

    fn visit_markup(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_top_level_element(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_heading(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_line(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_line_element(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_format(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_link(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_embed(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_link_target(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_list(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_unordered_list(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_numbered_list(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_checklist(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_line_quote(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_block_quote(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_macro_rule(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_plaintext(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_literal(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_link_body(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_link_alias_body(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_link_marker(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_embed_marker(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_macro_handle(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_inline_code(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_inline_raw_code(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_inline_math(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_inline_format(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_newline(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_horizontal_rule(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_line_quote_marker(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_block_quote_open(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_block_quote_close(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_macro_args(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_macro_body(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_heading_marker(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_code_block(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_math_block(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_checkbox(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_list_item_marker(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_numbered_item_marker(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }

    fn visit_assignment_marker(&mut self, node: &AstNode<'a>) -> Self::Output {
        todo!()
    }
}

pub struct AstToMarkdown {}

impl<'a> AstVisitor<'a> for AstToMarkdown {
    type Output = Vec<String>;
}
