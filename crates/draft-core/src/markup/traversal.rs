use pastey::paste;

use crate::{
    markup::{
        lex::{InlineFormat, ListItemKind},
        parse::{AstNode, SymbolKind},
        traversal_utils::Visitor,
    },
    unpack_token,
};

macro_rules! visits {
    ($name:ident $(,)?) => {
        paste! {
            fn [< visit_ $name >](&mut self, node: &AstNode<'a>);
        }
    };
}

macro_rules! visitor {
    ($name:ident, $body:expr $(,)?) => {
        paste! {
            #[inline(always)]
            fn [< visit_ $name >](&mut self, node: &AstNode<'a>) {
                ($body as Visitor<'a,_>)(self, node)
            }
        }
    };
}

macro_rules! emit {
    ($model:ident, $($arg:tt)*) => {
        $model.out.push_str(&format!($($arg)*))
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
    // Rules
    visits!(markup);
    visits!(top_level_element);
    visits!(heading);
    visits!(line);
    visits!(line_element);
    visits!(format);
    visits!(link);
    visits!(embed);
    visits!(paragraph);
    visits!(link_target);
    visits!(list);
    visits!(list_item);
    visits!(line_quote);
    visits!(block_quote);
    visits!(macro_rule);

    // Tokens
    visits!(plaintext);
    visits!(literal);
    visits!(link_body);
    visits!(link_alias_body);
    visits!(inferred_link);
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

    // Everything else
    visits!(none);
}

/// Emitted CSS obeys block-element-modifier (BEM) rules:
/// - **Block:** `.block`
/// - **Element:** `.block__element`
/// - **Modifier:** `.block--modifier` or `.block__element--modifier`
#[cfg(feature = "to-html")]
pub struct HtmlVisitor {
    out: String,
    in_pgraph: bool,
}

#[cfg(feature = "to-html")]
impl HtmlVisitor {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            out: String::new(),
            in_pgraph: false,
        }
    }
}

#[cfg(feature = "to-html")]
impl<'a> AstVisitor<'a> for HtmlVisitor {
    visitor!(none, |model: &mut AstToHtml, node| {
        match node.kind {
            super::parse::NodeKind::Rule(rule_kind) => todo!(),
            super::parse::NodeKind::Token(token) => todo!(),
        }
    });

    visitor!(paragraph, |model: &mut HtmlVisitor, node| {
        model.in_pgraph = true;
        emit!(model, "<p class='dt-pgraph'>");
        node[0]
            .iter()
            .filter(|&child| child.kind.as_rule_kind().is_some())
            .for_each(|child| {
                model.visit_line_element(child);
            });
        emit!(model, "</p>");
        model.in_pgraph = false;
    });

    visitor!(newline, |model: &mut HtmlVisitor, _| {
        emit!(model, " ");
    });

    visitor!(list, |model: &mut HtmlVisitor, node| { todo!() });

    visitor!(markup, |model: &mut HtmlVisitor, node| {});

    visitor!(heading, |model: &mut HtmlVisitor, node| {
        unpack_token!(node[0], HeadingMarker { depth });
        emit!(model, "<h{depth}>");
        model.visit_line(&node[1]);
        emit!(model, "</h{depth}>");
    });

    visitor!(format, |model: &mut HtmlVisitor, node| {
        unpack_token!(node[0], InlineFormat { ty });
        match ty {
            InlineFormat::BOLD => {
                emit!(model, "<b class='dt-bold'>");
                model.visit_paragraph(&node[1]);
                emit!(model, "</b>");
            }
            InlineFormat::HIGHLIGHT => {
                emit!(model, "<mark class='dt-hl'>");
                model.visit_paragraph(&node[1]);
                emit!(model, "</mark>");
            }
            InlineFormat::ITALIC => {
                emit!(model, "<i class='dt-italic'>");
                model.visit_paragraph(&node[1]);
                emit!(model, "</i>");
            }
            InlineFormat::STRIKETHROUGH => {
                emit!(model, "<s class='dt-rem'>");
                model.visit_paragraph(&node[1]);
                emit!(model, "</s>");
            }
            InlineFormat::UNDERLINE => {
                emit!(model, "<u class='dt-under'>");
                model.visit_paragraph(&node[1]);
                emit!(model, "</u>");
            }
            _ => panic!("Invalid format"),
        }
    });

    visitor!(horizontal_rule, |model: &mut HtmlVisitor, _| {
        emit!(model, "<hr>");
    });
}

/// Transforms an AST into Github-flavored Markdown (GFM)
#[cfg(feature = "to-markdown")]
pub struct MarkdownVisitor {
    out: String,
}

#[cfg(feature = "to-markdown")]
impl MarkdownVisitor {
    #[inline(always)]
    pub const fn new() -> Self {
        Self { out: String::new() }
    }
}

#[cfg(feature = "to-markdown")]
impl<'a> AstVisitor<'a> for MarkdownVisitor {
    visitor!(horizontal_rule, |model: &mut MarkdownVisitor, _| {
        emit!(model, "---");
    });

    visitor!(newline, |model: &mut MarkdownVisitor, node| {
        emit!(model, "\n");
    });

    visitor!(heading, |model: &mut MarkdownVisitor, node| {
        unpack_token!(node[0], HeadingMarker { depth });
        emit!(model, "{:#>1$} ", "", depth as usize);
        model.visit_line(&node[1]);
    });

    visitor!(format, |model: &mut MarkdownVisitor, node| {
        unpack_token!(node[0], InlineFormat { ty });
        match ty {
            InlineFormat::BOLD => {
                emit!(model, "**");
                model.visit_paragraph(&node[1]);
                emit!(model, "**");
            }
            InlineFormat::HIGHLIGHT => {
                emit!(model, "***"); // default to bold-italic
                model.visit_paragraph(&node[1]);
                emit!(model, "***");
            }
            InlineFormat::ITALIC => {
                emit!(model, "*");
                model.visit_paragraph(&node[1]);
                emit!(model, "*");
            }
            InlineFormat::STRIKETHROUGH => {
                emit!(model, "~~");
                model.visit_paragraph(&node[1]);
                emit!(model, "~~");
            }
            InlineFormat::UNDERLINE => {
                emit!(model, "<ins>");
                model.visit_paragraph(&node[1]);
                emit!(model, "</ins>");
            }
            _ => panic!("Invalid format"),
        }
    });
}
