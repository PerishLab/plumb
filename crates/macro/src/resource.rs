use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, LitStr};

pub fn expand(path: LitStr) -> Result<TokenStream, Error> {
    let held = path.value();
    if let Err(why) = anchored(&held) {
        return Err(Error::new(path.span(), why));
    }
    Ok(quote! {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", #held))
    })
}

fn anchored(held: &str) -> Result<(), String> {
    if held.is_empty() {
        return Err("a resource names no path".to_string());
    }
    if held.starts_with('/') || held.contains('\\') {
        return Err(format!("{held} is not a crate-relative resource"));
    }
    for part in held.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(format!("{held} leaves the crate carrying it"));
        }
    }
    Ok(())
}
