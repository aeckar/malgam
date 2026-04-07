/// A language syntax that can be compiled once.
pub trait Compile {
    type Output;

    /// Compile a given syntax; if already compiled, does nothing.
    fn compile(self) -> Self::Output;
}
