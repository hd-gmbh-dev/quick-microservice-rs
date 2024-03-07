pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the string representation
    // let mut input = syn::parse_macro_input!(input as syn::DeriveInput);
    // expand_derive_rcobj(&mut input)
    //     .unwrap_or_else(syn::Error::into_compile_error)
    //     .into()
    input
}
