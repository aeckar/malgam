//! `#[inline(always)]` should not be used except under extraordinary cirumstances (see `Tape`).
//! One should mark small functions that resolve to non-block expressions with `#[inline]`
//! to enable inlining from external crates.
//!
//! When applicable, functions should be marked `const`.
//! 
//! Import alias should only be used locally for readability, unless an `enum`
//! is used many times in the same file. Use of star import, except for `use crate::prelude::*`,
//! is discouraged.
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

/// Unpacks a struct-like enum variant from a value, asserting that
/// the value matches the expected variant.
///
/// This macro expands to a `let` binding with an `else` branch that panics
/// if the pattern does not match. It supports binding variant fields by
/// name, optional aliasing, and an optional `..` to ignore remaining fields.
/// 
/// # Examples
/// ```rust
/// unpack!(value, MyEnum::Variant { a, b: alias, .. });
/// ```
///
/// # Panics
/// Panics if the provided instance is not the expected variant.
#[macro_export]
macro_rules! unpack {
    ($instance:expr, $variant:pat) => {
        let $variant = $instance else {
            panic!("Unpack failed: Expected {}", stringify!($variant));
        };
    };
}
