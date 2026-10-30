use super::Walker;
use crate::consumption::lock::form::Form;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

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
        self.gap(
            mac.span(),
            "macro expansion is not inspected; its generated lock behavior is unknown".into(),
        );
        Form::Opaque
    }
}
