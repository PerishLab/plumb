use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemStruct, LitStr};

pub fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let body = value(args)?;
    let item: ItemStruct = syn::parse2(input)?;
    let name = &item.ident;
    let vis = &item.vis;
    let (params, types, clause) = item.generics.split_for_impl();
    Ok(quote! {
        #item
        impl #params #name #types #clause {
            #vis fn prefix() -> ::std::string::String {
                #body
            }
        }
    })
}

fn value(args: TokenStream) -> syn::Result<TokenStream> {
    if args.is_empty() {
        return Ok(quote! {
            ::std::env!("CARGO_PKG_NAME").to_uppercase().replace('-', "_")
        });
    }
    let held: LitStr = syn::parse2(args)?;
    let text = held.value();
    if !valid(&text) {
        return Err(syn::Error::new_spanned(
            held,
            "prefix must match [A-Z][A-Z0-9_]*",
        ));
    }
    Ok(quote! {
        ::std::string::String::from(#text)
    })
}

fn valid(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_uppercase()
        && chars.all(|char| char.is_ascii_uppercase() || char.is_ascii_digit() || char == '_')
}
