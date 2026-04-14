//! Modules should be imported internally using re-export.
pub mod config;
mod lexer;
mod lexer_utils;
mod parser;
mod parser_utils;
mod traversal;
mod traversal_utils;

pub mod lex {
    pub use super::{lexer::*, lexer_utils::*};
}

pub mod parse {
    pub use super::{parser::*, parser_utils::*};
}

pub mod visit {
    pub use super::{traversal::*, traversal_utils::*};
}
