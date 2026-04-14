//! `#[inline(always)]` should not be used except under extraordinary cirumstances (see `Tape`).
//! One should mark small functions that resolve to non-block expressions with `#[inline]`
//! to enable inlining from external crates.
//! 
//! When applicable, functions should be marked `const`.
#![feature(macro_metavar_expr)]
mod compile;
mod ext;
mod tape;

#[cfg(feature = "parse-markup")]
pub mod markup;

#[cfg(feature = "parse-data")]
pub mod data;

#[cfg(feature = "parse-expressions")]
pub mod expr;

#[cfg(feature = "macros")]
pub mod macros;

#[cfg(feature = "formatter")]
pub mod fmt;

pub mod prelude {
    pub use super::{compile::*, ext::*, tape::*};
}
