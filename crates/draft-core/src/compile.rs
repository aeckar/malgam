/// Data that can be compiled once.
/// 
/// This trait should be implemented for complex operations
/// where state is shared in the same `self` instance.
pub trait Compile {
    type Output;

    /// Compile a given syntax; if already compiled, does nothing.
    fn compile(self) -> Self::Output;
}
