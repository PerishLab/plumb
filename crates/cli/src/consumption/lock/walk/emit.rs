use super::Walker;
use crate::consumption::lock::form::{Family, Form};
use crate::consumption::lock::record::{Domain, Finding, Gap, Hold};
use proc_macro2::Span;

const CARRIERS: [&str; 4] = ["acquire", "manual", "permit", "scoped"];

impl Walker<'_> {
    pub fn finding(&self, span: Span, form: &'static str, role: &'static str) -> Finding {
        let start = span.start();
        Finding {
            path: self.parsed.shown.clone(),
            line: start.line,
            column: start.column + 1,
            item: self.item.clone(),
            form,
            role,
            lifecycle: None,
            call: None,
            kind: None,
            carrier: None,
            hold: None,
            domain: Domain::default(),
            via: Vec::new(),
            cfg: self.cfg.clone(),
            context: self.context(),
        }
    }

    pub fn context(&self) -> &'static str {
        if self.context == "production" && self.cfg.iter().any(|cfg| positive(cfg)) {
            return "test";
        }
        self.context
    }

    pub fn hold(&self, form: &'static str) -> Hold {
        Hold {
            form,
            ..Hold::default()
        }
    }

    pub fn push(&mut self, finding: Finding, at: Option<usize>) -> Option<usize> {
        let carrying = matches!(finding.role, "primitive" | "radicula")
            && finding
                .lifecycle
                .is_some_and(|lifecycle| CARRIERS.contains(&lifecycle));
        if carrying && !self.harvest.direct.contains_key(&self.item) {
            let evidence = format!(
                "{} at {}:{}",
                finding.call.clone().unwrap_or_default(),
                finding.path,
                finding.line
            );
            self.harvest.direct.insert(self.item.clone(), evidence);
        }
        if !self.unit.local {
            return None;
        }
        self.harvest.findings.push(finding);
        let index = self.harvest.findings.len() - 1;
        if let Some(at) = at {
            self.marks.insert(at, index);
        }
        Some(index)
    }

    pub fn text(&self, span: Span) -> String {
        let raw = self.parsed.text.get(span.byte_range()).unwrap_or_default();
        let words: Vec<&str> = raw.split_whitespace().collect();
        let joined = words.join(" ");
        if joined.chars().count() > 160 {
            let cut: String = joined.chars().take(157).collect();
            return format!("{cut}...");
        }
        joined
    }

    pub fn gap(&mut self, span: Span, reason: String) {
        if !self.unit.local {
            return;
        }
        let start = span.start();
        let text = self.text(span);
        self.harvest.gaps.push(Gap {
            path: self.parsed.shown.clone(),
            line: start.line,
            column: start.column + 1,
            item: self.item.clone(),
            reason,
            text,
        });
    }

    pub fn typed(
        &mut self,
        span: Span,
        shape: (&Form, Vec<String>),
        named: (String, &'static str),
    ) {
        let (form, via) = shape;
        let Some((family, found)) = locate(form) else {
            return;
        };
        let role = match family {
            Family::Radicula => "radicula",
            Family::Descriptor => "primitive",
            Family::Mutex if found.first() == Form::Unit => "zone",
            Family::Mutex => "data",
            Family::Semaphore => "zone",
            Family::Init => "init",
            _ => "atomic",
        };
        let mut finding = self.finding(span, named.1, role);
        finding.kind = Some(found.render());
        finding.carrier = Some(form.render());
        finding.domain.owner = Some(named.0);
        finding.via = via;
        self.push(finding, None);
    }

    pub fn domain(&self, receiver: &syn::Expr) -> Domain {
        use syn::spanned::Spanned;
        let core = core(receiver);
        let at = core as *const syn::Expr as usize;
        let origin = match core {
            syn::Expr::Path(path) => path
                .path
                .get_ident()
                .and_then(|ident| self.lookup(&ident.to_string()))
                .and_then(|binding| binding.origin.clone()),
            _ => None,
        };
        Domain {
            receiver: Some(self.text(receiver.span())),
            owner: self.owners.get(&at).cloned(),
            origin,
            key: None,
        }
    }
}

pub fn core(expr: &syn::Expr) -> &syn::Expr {
    match expr {
        syn::Expr::Reference(inner) => core(&inner.expr),
        syn::Expr::Paren(inner) => core(&inner.expr),
        syn::Expr::Group(inner) => core(&inner.expr),
        syn::Expr::Try(inner) => core(&inner.expr),
        syn::Expr::Unary(inner) => core(&inner.expr),
        syn::Expr::MethodCall(call) if transparent(&call.method.to_string()) => {
            core(&call.receiver)
        }
        _ => expr,
    }
}

pub fn transparent(name: &str) -> bool {
    matches!(
        name,
        "clone" | "as_ref" | "as_mut" | "borrow" | "deref" | "by_ref"
    )
}

fn positive(cfg: &str) -> bool {
    let words: Vec<&str> = cfg
        .split(|letter: char| !letter.is_alphanumeric() && letter != '_')
        .collect();
    words.contains(&"test") && !cfg.contains("not(test")
}

fn locate(form: &Form) -> Option<(Family, Form)> {
    for wanted in [
        Family::Radicula,
        Family::Descriptor,
        Family::Mutex,
        Family::Semaphore,
        Family::Init,
        Family::Atomic,
    ] {
        if let Some(found) = search(form, wanted) {
            return Some((wanted, found));
        }
    }
    None
}

fn search(form: &Form, wanted: Family) -> Option<Form> {
    if form.family() == Some(wanted) {
        return Some(form.clone());
    }
    match form {
        Form::Named(_, args) => args.iter().find_map(|arg| search(arg, wanted)),
        _ => None,
    }
}
