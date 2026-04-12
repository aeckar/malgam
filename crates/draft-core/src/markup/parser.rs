use crate::markup::lexer_utils::TokenKind as token;
use crate::markup::parse::AstNode;
use crate::markup::parser_utils::NodeMetadata as meta;
use crate::markup::parser_utils::RuleKind as rule;
use crate::{
    compile::Compile,
    markup::{lexer_utils::TokenSpan, parser_utils::AstNode as node},
    tape::Tape,
};

/// Since a zero-length input is also accepted, a match (even if partial)
/// will always be made. To check if the entire input is matched, check the root `end`.
struct Parser<'a> {
    // All tokens in the markup file.
    tokens: &'a [TokenSpan<'a>],
}

impl<'a> Compile for Parser<'a> {
    type Output = Result<'a>;

    fn compile(self) -> Self::Output {
        Rules::markup(Tape::new(self.tokens))
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

/// Declares a handler for the rule of the given name.
///
/// `body` is passed as a closure (as opposed to a block) to allow for full IntelliSense
/// and formatting.
macro_rules! rule {
    ($name:ident, $body:expr $(,)?) => {
        #[inline(always)]
        pub fn $name(tape: SpanTape<'a>) -> Result<'a> {
            ($body as Handler<'a>)(tape)
        }
    };
}

pub type SpanTape<'a> = Tape<'a, TokenSpan<'a>>;
pub type Result<'a> = Option<(node<'a>, SpanTape<'a>)>;
pub type Handler<'a> = fn(SpanTape<'a>) -> Option<(node<'a>, SpanTape<'a>)>;

/// Used to assemble the AST according to the following grammar:
/// ```ebnf
/// markup := topLevelElement*
///
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
///     | macro
///
/// format := InlineFormat & lineElement & InlineFormat
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
/// list := unorderedList | numberedList | checklist
/// unorderedList := (ListItemMarker & line)+
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
///
/// This parser, like `Lexer`, is hand-written to encourage a simple API and
/// optimal performance.
pub struct Rules;

impl<'a> Rules {
    rule!(markup, |mut tape| {
        let mut children = vec![];
        while let Some((child, jump)) = Self::top_level_element(tape) {
            children.push(child);
            tape = jump
        }
        Some((node::branch(rule::Markup, children, meta::None), tape))
    });

    rule!(top_level_element, |mut tape| {
        let (len, res) = token_options![tape; HorizontalRule, CodeBlock, MathBlock];
        if let Some((choice, child)) = res {
            return Some((
                node::branch(rule::TopLevelElement, vec![child], choice),
                tape,
            ));
        }
        for (choice, handler) in rule_options![len; list, heading, line_quote, block_quote] {
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
        if let Some(child) = node::try_token(token::HeadingMarker, &mut tape) {
            children.push(child);
            let (child, mut tape) = Self::line(tape)?;
            children.push(child);
            if let Some(child) = node::try_token(token::Newline, &mut tape) {
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
        if let Some(b) = node::try_token(token::Newline, &mut tape) {
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

    rule!(format, |mut tape| {
        let a = node::try_token(token::InlineFormat, &mut tape)?;
        let (b, mut tape) = Self::line_element(tape)?;
        let c = node::try_token(token::InlineFormat, &mut tape)?;
        Some((node::branch(rule::Format, vec![a, b, c], meta::None), tape))
    });

    rule!(link, |mut tape| {
        let a = node::try_token(token::LinkMarker, &mut tape)?;
        let (b, tape) = Self::link_target(tape)?;
        Some((node::branch(rule::Link, vec![a, b], meta::None), tape))
    });

    rule!(embed, |mut tape| {
        let a = node::try_token(token::EmbedMarker, &mut tape)?;
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

    rule!(list, |tape| {
        for (choice, handler) in rule_options![unordered_list, numbered_list, checklist] {
            if let Some((child, jump)) = handler(tape) {
                return Some((node::branch(rule::List, vec![child], choice), jump));
            }
        }
        None
    });

    rule!(unordered_list, |mut tape| {
        let mut children = vec![];
        while let Some(a) = node::try_token(token::ListItemMarker, &mut tape) {
            if let Some((b, jump)) = Self::line(tape) {
                children.push(node::branch(rule::None, vec![a, b], meta::None));
                tape = jump;
            } else {
                break;
            }
        }
        if children.is_empty() {
            return None;
        }
        Some((
            node::branch(rule::UnorderedList, children, meta::None),
            tape,
        ))
    });

    rule!(numbered_list, |mut tape| {
        let mut children = vec![];
        while let Some(a) = node::try_token(token::NumberedItemMarker, &mut tape) {
            if let Some((b, jump)) = Self::line(tape) {
                children.push(node::branch(rule::None, vec![a, b], meta::None));
                tape = jump;
            } else {
                break;
            }
        }
        if children.is_empty() {
            return None;
        }
        Some((node::branch(rule::NumberedList, children, meta::None), tape))
    });

    rule!(checklist, |mut tape| {
        let mut children = vec![];
        while let Some(a) = node::try_token(token::Checkbox, &mut tape) {
            if let Some((b, jump)) = Self::line(tape) {
                children.push(node::branch(rule::None, vec![a, b], meta::None));
                tape = jump;
            } else {
                break;
            }
        }
        if children.is_empty() {
            return None;
        }
        Some((node::branch(rule::Checklist, children, meta::None), tape))
    });

    rule!(line_quote, |mut tape| {
        let a = node::try_token(token::LineQuoteMarker, &mut tape)?;
        let (b, tape) = Self::link_target(tape)?;
        Some((node::branch(rule::LineQuote, vec![a, b], meta::None), tape))
    });

    rule!(block_quote, |mut tape| {
        let a = node::try_token(token::BlockQuoteOpen, &mut tape)?;
        let choice: u8;
        let child_b = if let Some(child_b) = node::try_token(token::Newline, &mut tape) {
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
        let d = node::try_token(token::BlockQuoteClose, &mut tape)?;
        Some((
            node::branch(rule::BlockQuote, vec![a, b, c, d], meta::None),
            tape,
        ))
    });

    rule!(macro_rule, |mut tape| {
        let a = node::try_token(token::MacroHandle, &mut tape)?;
        let children_b: Vec<AstNode<'a>> = node::try_token(token::MacroArgs, &mut tape)
            .into_iter()
            .collect();
        let is_present = !children_b.is_empty();
        let b = node::new(rule::None, children_b, a.end, meta::IsPresent(is_present));
        let mut children_c = vec![];
        while let Some(child_c) = node::try_token(token::MacroBody, &mut tape) {
            children_c.push(child_c);
        }
        let is_present = !children_c.is_empty();
        let c = node::new(rule::None, children_c, b.end, meta::IsPresent(is_present));
        Some((node::branch(rule::Macro, vec![a, b, c], meta::None), tape))
    });
}
