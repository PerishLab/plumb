use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod parse;

#[proc_macro_derive(Cascade, attributes(cascade))]
pub fn cascade(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as DeriveInput);
    parse::expand(item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
