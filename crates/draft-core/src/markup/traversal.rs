use std::sync::OnceLock;

use pastey::paste;
use regex::Regex;

use crate::markup::{lex::{InlineFormat, Token}, parser_utils::AstNode};

static YOUTUBE_LINK: OnceLock<Regex> = OnceLock::new();
static MEDIA_LINK: OnceLock<Regex> = OnceLock::new();

/// Returns the regex for a YouTube video link.
pub fn get_yt_link() -> &'static Regex {
    YOUTUBE_LINK.get_or_init(|| {
        Regex::new(r"(?:https?://)?(?:www\.)?(?:youtube\.com/watch\?.*v=|youtu\.be/)([\w-]{11})(?:[&?]\S*)?")
            .expect("Invalid YouTube video regex")
    })
}

/// Returns the regex for a media file link.
pub fn get_media_link() -> &'static Regex {
    MEDIA_LINK.get_or_init(|| {
        Regex::new(r"\.(csv|jpg|jpeg|png|webp|svg|mp3|ogg|opus|mp4|webm)(?:\?.*)?(?:#.*)?$")
            .expect("Invalid media file regex")
    })
}

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

macro_rules! fallthrough {
    ($name:ident $(,)?) => {
        paste! {
            fn [< visit_ $name >](&mut self, node: &AstNode<'a>) {
                self.visit_none(node);
            }
        }
    };
}

macro_rules! emit {
    ($model:ident, $($arg:tt)* $(,)?) => {
        $model.out.push_str(&format!($($arg)*))
    };
}

macro_rules! unpack {
    ($instance:expr, $variant:path { $($field:ident),* }) => {
        let $variant { $($field),* , .. } = $instance.kind.token().unwrap() else {
            panic!("Unpack failed: Expected {}", stringify!($variant));
        };
    };
}

pub type Visitor<'a, T: AstVisitor<'a>> = fn(&mut T, node: &AstNode<'a>);

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

    // Everything else
    visits!(none);
}

pub struct AstToHtml {
    out: String,
}

// todo integrate arena alloc
impl<'a> AstVisitor<'a> for AstToHtml {
    fallthrough!(list);
    fallthrough!(line);

    visitor!(none, |model|)

    visitor!(markup, |model: &mut AstToHtml, node| {
        model.out.push(ch);
    });

    visitor!(heading, |model: &mut AstToHtml, node| {
        unpack!(node.children[0], Token::HeadingMarker { depth });
        emit!(model, "<h{depth}>");
        model.visit_line(&node.children[1]);
        emit!(model, "</h{depth}>");
    });

    visitor!(format, |model: &mut AstToHtml, node| {
        unpack!(node.children[0], Token::InlineFormat { ty });
        match ty {
            InlineFormat::Bold => {
                emit!(model, "<>")
                model.visit_
            }
        }
    });
}

pub struct AstToMarkdown {
    out: String,
}

impl<'a> AstVisitor<'a> for AstToMarkdown {
    
}
