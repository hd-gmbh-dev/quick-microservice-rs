/// Trait for comparing two values for partial equality.
pub trait PartialEqual<'a, R> {
    /// Compares two values for equality.
    fn partial_equal(&'a self, r: &'a R) -> bool;
}

impl<'a, L, R> PartialEqual<'a, R> for L
where
    &'a L: Into<R> + 'a,
    R: PartialEq,
{
    fn partial_equal(&'a self, r: &'a R) -> bool {
        &self.into() == r
    }
}
