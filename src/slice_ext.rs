use crate::char_ext::CharExt;

pub trait SliceExt {
    /// Returns a subslice with leading and trailing white space removed,
    /// according to the compiler.
    fn trim_ws(&self) -> Self;
}

impl SliceExt for &[u8] {
    #[inline(always)]
    fn trim_ws(&self) -> Self {
        let mut bytes = *self;
        while let [first, rest @ ..] = bytes {
            // peel off front
            if first.is_ws() {
                bytes = rest;
            } else {
                break;
            }
        }
        while let [rest @ .., last] = bytes {
            // peel off back
            if last.is_ws() {
                bytes = rest;
            } else {
                break;
            }
        }
        bytes
    }
}
