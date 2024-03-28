pub trait PartialEqual<'a, R> {
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
