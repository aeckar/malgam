pub mod parser;
pub mod token;
pub mod tape;
pub mod macros;

mod char_ext;
mod slice_ext;
mod util;

pub mod prelude {
    pub use crate::{char_ext::*, slice_ext::*, util::*};
}