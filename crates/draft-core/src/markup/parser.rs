use std::vec;

use crate::{
    markup::{
        lex::{ListItemKind, ListItemPos, Token, TokenKind as token, TokenSpan},
        parse::{
            AstNode, AstNode as node, Handler, NodeKind, NodeMetadata as meta, Result,
            RuleKind as rule, TokenStream,
        },
    }, prelude::*, unpack, unpack_token
};

/// Since a zero-length input is also accepted, a match (even if partial)
/// will always be made. To check if the entire input is matched, check the root `end`.
pub struct Parser<'a> {
    // All tokens in the markup file.
    tokens: &'a [TokenSpan<'a>],
}

impl<'a> Compile for Parser<'a> {
    type Output = Result<'a>;

    fn compile(self) -> Self::Output {
        Grammar::markup(Tape::new(self.tokens))
    }
}

/// Enumerates the rule names given as an array of tuples, each containing:
/// - The index of the element in the array as `Choice` metadata
/// - The rule handler (in `Rules`)
///
/// Returns `[(choice, handler)]`, or `handler` if a single name is given.
macro_rules! rule_options {
    // Without offset
    ($($name:ident),* $(,)?) => {
        [
            $(
                (
                    meta::Choice(${index()} as u8),
                    Self::$name as Handler<'a>
                )
            ),*
        ]
    };

    // With offset
    ($offset:expr; $($name:ident),* $(,)?) => {
        [
            $(
                (
                    meta::Choice((${index()} + $offset) as u8),
                    Self::$name as Handler<'a>
                )
            ),*
        ]
    };
}

/// Queries the next token span in the tape, if one exists.
/// If so, it is matched against each of the kind of tokens given.
///
/// On a successful match, `tape.pos` is incremented by 1, and the second member
/// of the returned tuple is populated with:
/// - The index of the chosen kind
/// - The AST node
///
/// The first member is the number of kinds passed to this macro.
///
/// Returns `(len, Option(choice, node))`.
macro_rules! token_options {
    ($tape:expr; $($name:ident),* $(,)?) => {
        {
            let tokens = [$(token::$name),*];
            if let Some(span) = $tape.peek() {
                let peek = span.token.kind();
                let choice = tokens.iter().position(|t| *t == peek);
                if let Some(choice) = choice {
                    $tape.adv();
                    (tokens.len(), Some((meta::Choice(choice as u8), node::token(span))))
                } else {
                    (tokens.len(), None)
                }
            } else {
                (tokens.len(), None)
            }
        }
    };
}

/// Queries the next token span in the tape, if one exists.
///
/// Returns `Option(node)`.
macro_rules! try_token {
    ($tape:expr, $name:ident $(,)?) => {
        node::try_token(token::$name, &mut $tape)
    };
}

/// Queries the next token span in the tape, if one exists.
///
/// If the match succeeds, the the first member of the returned tuple is `true`,
/// or `false` otherwise. The second member is always a vector containing
/// the single matched node, or an empty list if the match failed.
///
/// Returns `(is_present, children)`
macro_rules! optional_token {
    ($tape:expr, $name:ident $(,)?) => {{
        let children: Vec<AstNode<'a>> = try_token!($tape, $name).into_iter().collect();
        let is_present = meta::IsPresent(!children.is_empty());
        (is_present, children)
    }};
}

/// Declares a handler for the rule of the given name.
///
/// `body` is passed as a closure (as opposed to a block) to allow for full IntelliSense
/// and formatting.
macro_rules! rule {
    ($name:ident, $body:expr $(,)?) => {
        #[inline(always)]
        pub fn $name(tape: TokenStream<'a>) -> Result<'a> {
            ($body as Handler<'a>)(tape)
        }
    };
}

/// Used to assemble the AST according to the following grammar:
/// ```ebnf
/// markup := topLevelElement*
///
/// topLevelElement := Newline
///     | HorizontalRule
///     | CodeBlock
///     | MathBlock
///     | list
///     | paragraph
///     | heading
///     | lineQuote
///     | blockQuote
/// heading := HeadingMarker
///     & line
///     & Newline
///
/// # stops at Newline & Newline
/// # consumes continuations that whose bullet type could not be inferred
/// paragraph := ContinuationMarker? & (Newline | lineElement)+
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
///     | macro
///
/// format := InlineFormat & paragraph & InlineFormat
/// link := LinkMarker & linkTarget
/// embed := EmbedMarker & linkTarget
/// linkTarget := LinkBody | LinkAliasBody
///
/// lineQuote := LineQuoteMarker & line
/// blockQuote := BlockQuoteOpen
///     & (Newline | line)
///     & topLevelElement+
///     & BlockQuoteClose
///
/// # for clarity, no empty lines between list items
/// list := listItem & (Newline* & listItem)*   # may be multiple lists, split during second pass
/// listItem := (ListItemMarker | NumberedItemMarker | Checkbox) & paragraph
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
///
/// This parser, like `Lexer`, is hand-written to encourage a simple API and
/// optimal performance.
///
/// It is imperative to keep the DSL-like macro API
/// internal as opposed to transferring it to a library to ensure all project constraints,
/// including performance. This applies to traversal as well.
///
/// Macros for common operations enable rapid iteration for changes in the EBNF.
///
/// Any caching/indexing should occur in the LSP itself, not due to the parser.
pub struct Grammar;

impl<'a> Grammar {
    rule!(markup, |mut tape| {
        let mut children = vec![];
        while let Some((child, jump)) = Self::top_level_element(tape) {
            children.push(child);
            tape = jump
        }
        Some((node::branch(rule::Markup, children, meta::None), tape))
    });

    rule!(top_level_element, |mut tape| {
        let (len, res) = token_options![tape; Newline, HorizontalRule, CodeBlock, MathBlock];
        if let Some((choice, child)) = res {
            return Some((
                node::branch(rule::TopLevelElement, vec![child], choice),
                tape,
            ));
        }
        for (choice, handler) in
            rule_options![len; paragraph, list, heading, line_quote, block_quote]
        {
            if let Some((child, jump)) = handler(tape) {
                return Some((
                    node::branch(rule::TopLevelElement, vec![child], choice),
                    jump,
                ));
            }
        }
        None
    });

    rule!(heading, |mut tape| {
        let mut children = vec![];
        if let Some(child) = try_token!(tape, HeadingMarker) {
            children.push(child);
            let (child, mut tape) = Self::line(tape)?;
            children.push(child);
            if let Some(child) = try_token!(tape, Newline) {
                children.push(child);
                return Some((node::branch(rule::Heading, children, meta::None), tape));
            }
        }
        None
    });

    rule!(line, |mut tape| {
        let mut children_a = vec![];
        while let Some((child_a, jump)) = Self::line_element(tape) {
            children_a.push(child_a);
            tape = jump;
        }
        if children_a.is_empty() {
            return None;
        }
        let a = node::branch(rule::None, children_a, meta::None);
        if let Some(b) = try_token!(tape, Newline) {
            return Some((node::branch(rule::Line, vec![a, b], meta::None), tape));
        }
        None
    });

    rule!(line_element, |mut tape| {
        let (len, res) =
            token_options![tape; Plaintext, InlineCode, InlineMath, InlineRawCode, Literal];
        if let Some((choice, child)) = res {
            return Some((node::branch(rule::LineElement, vec![child], choice), tape));
        }
        for (choice, handler) in rule_options![len; format, link, embed, macro_rule] {
            if let Some((child, jump)) = handler(tape) {
                return Some((node::branch(rule::LineElement, vec![child], choice), jump));
            }
        }
        None
    });

    rule!(paragraph, |mut tape| {
        let mut children = vec![];
        loop {
            let (choice, child_a) = if let Some(child_a) = try_token!(tape, Newline) {
                if tape
                    .peek()
                    .is_some_and(|span| span.token.kind() == token::Newline)
                {
                    break;
                }
                (0, child_a)
            } else if let Some((child_a, jump)) = Self::line_element(tape) {
                tape = jump;
                (1, child_a)
            } else {
                break;
            };
            let child = node::branch(rule::None, vec![child_a], meta::Choice(choice));
            children.push(child);
        }
        if children.is_empty() {
            return None;
        }
        Some((node::branch(rule::None, children, meta::None), tape))
    });

    rule!(format, |mut tape| {
        let a = try_token!(tape, InlineFormat)?;
        let (b, mut tape) = Self::paragraph(tape)?;
        let closer = tape.next().filter(|span| matches!(span.token, Token::InlineFormat { twin_pos, .. } if twin_pos == a.start))?;
        let c = AstNode {
            start: closer.start,
            end: closer.end,
            parent: None,
            children: vec![],
            meta: meta::None,
            kind: NodeKind::Token(closer.token),
        };
        Some((node::branch(rule::Format, vec![a, b, c], meta::None), tape))
    });

    rule!(link, |mut tape| {
        let a = try_token!(tape, LinkMarker)?;
        let (b, tape) = Self::link_target(tape)?;
        Some((node::branch(rule::Link, vec![a, b], meta::None), tape))
    });

    rule!(embed, |mut tape| {
        let a = try_token!(tape, EmbedMarker)?;
        let (b, tape) = Self::link_target(tape)?;
        Some((node::branch(rule::Embed, vec![a, b], meta::None), tape))
    });

    rule!(link_target, |mut tape| {
        let (_, res) = token_options![tape; LinkBody, LinkAliasBody];
        if let Some((choice, child)) = res {
            return Some((node::branch(rule::LinkTarget, vec![child], choice), tape));
        }
        None
    });

    rule!(list, |mut tape| {
        let mut children_a = vec![];
        let node = node::new(rule::List, vec![], tape.peek()?.start, meta::None);
        while let Some((child_a, jump)) = Self::list_item(tape, &node) {
            children_a.push(child_a);
            tape = jump;
        }
        unpack!(children_a.last()?.meta, meta::ListItem { kind, .. });
        
        let a = node::branch(rule::None, children_a, meta::None);
        Some((node::branch(rule::List, vec![a], meta::None), tape))
    });

    pub fn list_item(mut tape: TokenStream<'a>, parent: &AstNode<'a>) -> Result<'a> {
        let mut a = try_token!(tape, ListItemMarker)?;
        unpack_token!(
            a,
            ListItemMarker {
                indent: indent_a,
                kind: kind_a
            }
        );
        let kind_a = if kind_a == ListItemKind::Continuation {
            unpack_token!(
                parent.children.iter().rev().find(|node| {
                    matches!(node.kind.token().unwrap(),
                        Token::ListItemMarker { indent, kind }
                        if indent == indent_a && kind != ListItemKind::Continuation
                    )
                })?,
                ListItemMarker { kind, .. }
            );
            kind
        } else {
            kind_a
        };
        if parent.children.is_empty() {
            let mut pos = ListItemPos::Any.bits();
            unpack_token!(
                parent.children.last().unwrap(),
                ListItemMarker { indent, .. }
            );
            if indent > indent_a {
                pos |= ListItemPos::First.bits();
            }
            a.meta = meta::ListItem {
                kind: kind_a,
                pos: ListItemPos::from_bits(pos).unwrap(),
            };
        } else {
            a.meta = meta::ListItem {
                kind: kind_a,
                pos: ListItemPos::Any,
            };
        }
        let (b, tape) = Self::paragraph(tape)?;
        Some((node::branch(rule::ListItem, vec![a, b], meta::None), tape))
    }

    rule!(line_quote, |mut tape| {
        let a = try_token!(tape, LineQuoteMarker)?;
        let (b, tape) = Self::link_target(tape)?;
        Some((node::branch(rule::LineQuote, vec![a, b], meta::None), tape))
    });

    rule!(block_quote, |mut tape| {
        let a = try_token!(tape, BlockQuoteOpen)?;
        let choice: u8;
        let child_b = if let Some(child_b) = try_token!(tape, Newline) {
            choice = 0;
            child_b
        } else {
            choice = 1;
            let (child_b, jump) = Self::line(tape)?;
            tape = jump;
            child_b
        };
        let b = node::branch(rule::None, vec![child_b], meta::Choice(choice));
        let mut children_c = vec![];
        while let Some((child_c, jump)) = Self::top_level_element(tape) {
            children_c.push(child_c);
            tape = jump;
        }
        if children_c.is_empty() {
            return None;
        }
        let c = node::branch(rule::None, children_c, meta::None);
        let d = try_token!(tape, BlockQuoteClose)?;
        Some((
            node::branch(rule::BlockQuote, vec![a, b, c, d], meta::None),
            tape,
        ))
    });

    rule!(macro_rule, |mut tape| {
        let a = try_token!(tape, MacroHandle)?;
        let (is_present, children_b) = optional_token!(tape, MacroArgs);
        let b = node::new(rule::None, children_b, a.end, is_present);
        let mut children_c = vec![];
        while let Some(child_c) = try_token!(tape, MacroBody) {
            children_c.push(child_c);
        }
        let is_present = !children_c.is_empty();
        let c = node::new(rule::None, children_c, b.end, meta::IsPresent(is_present));
        Some((node::branch(rule::Macro, vec![a, b, c], meta::None), tape))
    });
}
