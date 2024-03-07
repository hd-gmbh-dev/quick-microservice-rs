use proc_macro::TokenStream;

mod cheap_clone;

#[proc_macro_derive(CheapClone)]
pub fn cheap_clone(item: TokenStream) -> TokenStream {
    cheap_clone::expand(item)
}
