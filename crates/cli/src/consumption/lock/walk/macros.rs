use super::Walker;
use super::method::SUSPECT;
use crate::consumption::lock::form::Form;
use proc_macro2::{TokenStream, TokenTree};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

const TOKENS: [&str; 8] = [
    "Mutex",
    "RwLock",
    "ReentrantMutex",
    "Semaphore",
    "flock",
    "radicula_lock",
    "FileExt",
    "Flock",
];

const PARSED: [&str; 22] = [
    "vec",
    "format",
    "print",
    "println",
    "eprint",
    "eprintln",
    "write",
    "writeln",
    "panic",
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "dbg",
    "todo",
    "unreachable",
    "unimplemented",
    "format_args",
    "anyhow",
    "bail",
];

impl Walker<'_> {
    pub fn mac(&mut self, mac: &syn::Macro) -> Form {
        let name = mac
            .path
            .segments
            .last()
            .map(|last| last.ident.to_string())
            .unwrap_or_default();
        if name == "include" {
            self.gap(
                mac.span(),
                "generated source pulled in by include! is not read".into(),
            );
            return Form::Opaque;
        }
        if PARSED.contains(&name.as_str()) {
            let parser = Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
            if let Ok(list) = mac.parse_body_with(parser) {
                for expr in &list {
                    self.infer(expr);
                }
                return Form::Opaque;
            }
        }
        self.hidden(mac, "lock tokens inside a macro body that is not parsed");
        Form::Opaque
    }

    pub fn hidden(&mut self, mac: &syn::Macro, reason: &str) {
        if suspicious(&mac.tokens) {
            self.gap(mac.span(), reason.to_string());
        }
    }
}

fn suspicious(tokens: &TokenStream) -> bool {
    tokens.clone().into_iter().any(|tree| match tree {
        TokenTree::Ident(ident) => {
            let word = ident.to_string();
            SUSPECT.contains(&word.as_str()) || TOKENS.contains(&word.as_str())
        }
        TokenTree::Group(group) => suspicious(&group.stream()),
        _ => false,
    })
}
