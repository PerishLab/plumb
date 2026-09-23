use proc_macro::TokenStream;

mod catalogue;
use syn::{DeriveInput, ItemFn, LitStr, parse_macro_input};

mod parse;
mod resource;
mod trace;

#[proc_macro_derive(Cascade, attributes(cascade))]
pub fn cascade(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as DeriveInput);
    parse::expand(item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro]
pub fn catalogue(input: TokenStream) -> TokenStream {
    catalogue::expand(input.into(), proc_macro2::Span::call_site())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn resource(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as LitStr);
    resource::expand(path)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn span(args: TokenStream, input: TokenStream) -> TokenStream {
    let name = parse_macro_input!(args as LitStr);
    let item = parse_macro_input!(input as ItemFn);
    trace::expand(name, item).into()
}
