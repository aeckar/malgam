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
    pub use super::compile::*;
    pub use super::ext::*;
    pub use super::tape::*;
}
