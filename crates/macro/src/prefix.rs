use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

pub fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    if !args.is_empty() {
        return Err(syn::Error::new_spanned(args, "prefix takes no arguments"));
    }
    let item: ItemStruct = syn::parse2(input)?;
    let name = &item.ident;
    let vis = &item.vis;
    let (params, types, clause) = item.generics.split_for_impl();
    Ok(quote! {
        #item
        impl #params #name #types #clause {
            #vis fn prefix() -> ::std::string::String {
                ::std::env!("CARGO_PKG_NAME").to_uppercase().replace('-', "_")
            }
        }
    })
}
