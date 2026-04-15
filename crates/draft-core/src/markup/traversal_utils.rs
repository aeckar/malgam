use indoc::formatdoc;

use crate::markup::{parse::AstNode, visit::AstVisitor};

// todo yt link
// todo media link

pub type Visitor<'a, T: AstVisitor<'a>> = fn(&mut T, node: &AstNode<'a>);

#[cfg(feature = "to-html")]
pub fn media_html(tag: &str, url: &str) -> String {
    formatdoc! {"
        <{tag} src='{url}' controls>\
            <span class='dt-error'>Your browser does not support the &lt;$tag&gt; tag.</span>\
        </{tag}>\
    "}
}
