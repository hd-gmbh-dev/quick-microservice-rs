use quote::quote;

fn expand_impl(ast: syn::ExprTuple) -> syn::Result<proc_macro2::TokenStream> {
    eprintln!("TODO: implement M2M, {ast:#?}");
    Ok(quote!())
}

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::ExprTuple);
    expand_impl(ast)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
