mod compile;
pub mod data;
pub mod expr;
mod ext;
pub mod markup;
mod tape;

pub mod prelude {
    pub use crate::compile::*;
    pub use crate::ext::*;
    pub use crate::tape::*;
}
