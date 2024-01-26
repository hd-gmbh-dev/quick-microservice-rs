use proc_macro::TokenStream;

mod m2m;
mod o2m;
mod o2o;

#[proc_macro]
pub fn m2m(item: TokenStream) -> TokenStream {
    m2m::expand(item)
}

#[proc_macro]
pub fn o2m(item: TokenStream) -> TokenStream {
    o2m::expand(item)
}

#[proc_macro]
pub fn o2o(item: TokenStream) -> TokenStream {
    o2o::expand(item)
}
