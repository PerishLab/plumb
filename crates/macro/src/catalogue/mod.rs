mod law;
mod walk;

use proc_macro2::TokenStream;
use quote::quote;
use syn::Error;

pub fn expand(input: TokenStream, span: proc_macro2::Span) -> Result<TokenStream, Error> {
    if !input.is_empty() {
        return Err(Error::new(
            span,
            "a catalogue reads the rules beside it and takes no arguments",
        ));
    }
    let root = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| Error::new(span, "a catalogue is read from the crate carrying it"))?;
    let root = std::path::Path::new(&root).join("rules");
    let held = walk::read(&root).map_err(|why| Error::new(span, why))?;
    law::judge(&held).map_err(|why| Error::new(span, why))?;
    let carried = held.files.iter().map(|(name, _)| {
        let held = format!("rules/{name}");
        quote! {
            (#held, include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/rules/", #name)))
        }
    });
    Ok(quote! {
        &[#(#carried),*]
    })
}
