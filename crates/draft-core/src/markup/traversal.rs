use std::sync::OnceLock;

use indoc::formatdoc;
use pastey::paste;

use crate::markup::{
    lex::{InlineFormat as fmt, Token},
    parse::{AstNode, NodeKind, SymbolKind},
    visit::ListItemKind,
};

// todo move all this to utils

static YOUTUBE_LINK: OnceLock<Regex> = OnceLock::new();
static MEDIA_LINK: OnceLock<Regex> = OnceLock::new();

/// Returns the regex for a YouTube video link.
fn get_yt_link() -> &'static Regex {
    YOUTUBE_LINK.get_or_init(|| {
        Regex::new(r"(?:https?://)?(?:www\.)?(?:youtube\.com/watch\?.*v=|youtu\.be/)([\w-]{11})(?:[&?]\S*)?")
        .expect("Invalid YouTube video regex")
    })
}

/// Returns the regex for a media file link.
fn get_media_link() -> &'static Regex {
    MEDIA_LINK.get_or_init(|| {
        Regex::new(r"\.(csv|jpg|jpeg|png|webp|svg|mp3|ogg|opus|mp4|webm)(?:\?.*)?(?:#.*)?$")
            .expect("Invalid media file regex")
    })
}

fn media_html(tag: &str, url: &str) -> String {
    formatdoc! {"
        <{tag} src='{url}' controls>\
            <span class='dt-error'>Your browser does not support the &lt;$tag&gt; tag.</span>\
        </{tag}>\
    "}
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

macro_rules! emit {
    ($model:ident, $($arg:tt)* $(,)?) => {
        $model.out.push_str(&format!($($arg)*))
    };
}

macro_rules! unpack {
    ($instance:expr, $variant:ident { $($field:ident),* }) => {
        let Token::$variant { $($field),* , .. } = $instance.kind.token().unwrap() else {
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
    in_pgraph: bool,
}

impl AstToHtml {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            out: String::new(),
            in_pgraph: false,
        }
    }
}

// -- bem

/*
Block: .block
Element: .block__element
Modifier: .block--modifier or .block__element--modifier  */

// todo integrate arena alloc
impl<'a> AstVisitor<'a> for AstToHtml {
    // fallthrough none
    // fallthrough line

    visitor!(paragraph, |model: &mut AstToHtml, node| {
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

    visitor!(newline, |model: &mut AstToHtml, _| {
        emit!(model, " ");
    });

    visitor!(list, |model: &mut AstToHtml, node| {
        let mut prev = ListItemKind::Continuation; // panics if used to get tag
        for child in node.iter() {
            let marker = &child[0];
            let kinds = vec![];
            let kind = ListItemKind::from_token(marker.kind.token().unwrap());
            loop {
                if kinds.is_empty() {
                    emit!(model, kind.open_tag_or(prev.unwrap()));
                }
            }
            prev = Some(kind);
        }
    });

    visitor!(markup, |model: &mut AstToHtml, node| {});

    visitor!(heading, |model: &mut AstToHtml, node| {
        unpack!(node[0], HeadingMarker { depth });
        emit!(model, "<h{depth}>");
        model.visit_line(&node[1]);
        emit!(model, "</h{depth}>");
    });

    visitor!(format, |model: &mut AstToHtml, node| {
        unpack!(node[0], InlineFormat { ty });
        match ty {
            fmt::Bold => {
                emit!(model, "<b class='dt-bold'>");
                model.visit_paragraph(&node[1]);
                emit!(model, "</b>");
            }
            fmt::Highlight => {
                emit!(model, "<mark class='dt-hl'>");
                model.visit_paragraph(&node[1]);
                emit!(model, "</mark>");
            }
            fmt::Italic => {
                emit!(model, "<i class='dt-italic'>");
                model.visit_paragraph(&node[1]);
                emit!(model, "</i>");
            }
            fmt::Strikethrough => {
                emit!(model, "<s class='dt-rem'>");
                model.visit_paragraph(&node[1]);
                emit!(model, "</s>");
            }
            fmt::Underline => {
                emit!(model, "<u class='dt-under'>");
                model.visit_paragraph(&node[1]);
                emit!(model, "</u>");
            }
        }
    });
}

pub struct AstToMarkdown {
    out: String,
}

impl AstToMarkdown {
    #[inline(always)]
    pub const fn new() -> Self {
        Self { out: String::new() }
    }
}

impl<'a> AstVisitor<'a> for AstToMarkdown {}
