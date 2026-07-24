use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr};

pub fn expand(name: LitStr, item: ItemFn) -> TokenStream {
    let attrs = &item.attrs;
    let vis = &item.vis;
    let sig = &item.sig;
    let block = &item.block;
    if sig.asyncness.is_some() {
        quote! {
            #(#attrs)* #vis #sig {
                ::plumb::context::Context::current()
                    .with(::plumb::trace::Span::open(#name))
                    .carry(async move #block)
                    .await
            }
        }
    } else {
        quote! {
            #(#attrs)* #vis #sig {
                let _entered = ::plumb::context::Context::current()
                    .with(::plumb::trace::Span::open(#name))
                    .enter();
                #block
            }
        }
    }
}
