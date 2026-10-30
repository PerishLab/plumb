use super::Walker;
use super::method::{Site, detail};
use crate::consumption::lock::form::{Family, Form, family};
use crate::consumption::lock::model::Fields;
use std::collections::BTreeMap;
use syn::spanned::Spanned;

const FLOCK: [&str; 4] = [
    "libc::flock",
    "nix::fcntl::flock",
    "rustix::fs::flock",
    "nix::fcntl::Flock::lock",
];

const STDIO: [(&str, &str); 3] = [
    ("std::io::stdin", "std::io::Stdin"),
    ("std::io::stdout", "std::io::Stdout"),
    ("std::io::stderr", "std::io::Stderr"),
];

const RADICULA: [(&str, &str); 20] = [
    ("lock", "manual"),
    ("attempt", "manual"),
    ("unlock", "release"),
    ("release", "release"),
    ("memory", "factory"),
    ("files", "factory"),
    ("redis", "factory"),
    ("connect", "factory"),
    ("gate", "gate"),
    ("shared", "gate"),
    ("wait", "gate"),
    ("tokio", "gate"),
    ("asynchronous", "gate"),
    ("acquire", "permit"),
    ("guard", "scoped"),
    ("with", "scoped"),
    ("hold", "scoped"),
    ("run", "scoped"),
    ("attach", "scoped"),
    ("during", "scoped"),
];

impl Walker<'_> {
    pub fn call(&mut self, call: &syn::ExprCall, at: usize) -> Form {
        let args: Vec<Form> = call.args.iter().map(|arg| self.infer(arg)).collect();
        let syn::Expr::Path(func) = &*call.func else {
            self.infer(&call.func);
            return Form::Opaque;
        };
        if func
            .path
            .get_ident()
            .is_some_and(|ident| self.lookup(&ident.to_string()).is_some())
        {
            return Form::Opaque;
        }
        let Some(path) = self.resolve(&func.path) else {
            return Form::Opaque;
        };
        let site = Site {
            span: func.span(),
            at,
            receiver: call.args.first(),
            args: call.args.iter().collect(),
            name: path.rsplit("::").next().unwrap_or_default().to_string(),
        };
        if path.starts_with("radicula_lock::") {
            let parent = path
                .rsplit_once("::")
                .map(|(head, _)| head.to_string())
                .unwrap_or_default();
            let own = Form::Named(parent, Vec::new());
            let whole = args.first().cloned().unwrap_or(Form::Opaque);
            return self.radicula(&site, &path, (&own, &whole));
        }
        if FLOCK.contains(&path.as_str())
            || (path.ends_with("fcntl") && self.text(call.span()).contains("F_SETLK"))
        {
            let whole = args.first().cloned().unwrap_or(Form::Opaque);
            self.emit(
                &site,
                (&Form::Opaque, &whole),
                detail("primitive", "acquire", path, Some("handle")),
            );
            return Form::Opaque;
        }
        if let Some((_, stdio)) = STDIO.iter().find(|(name, _)| *name == path) {
            return Form::Named(stdio.to_string(), Vec::new());
        }
        if let Some(output) = self.constructed(&site, &path, &args) {
            return output;
        }
        if let Some(function) = self.model.functions.get(&path) {
            let output = function.output.clone();
            self.edge(
                &Site {
                    receiver: None,
                    ..site
                },
                path,
            );
            return output;
        }
        if self.model.structs.contains_key(&path) {
            return Form::Named(path, Vec::new());
        }
        wrapped(&self.model.structs, &path, args)
    }

    fn constructed(&mut self, site: &Site, path: &str, args: &[Form]) -> Option<Form> {
        let (parent, last) = path.rsplit_once("::")?;
        let kind = family(parent)?;
        let first = args.first().cloned().unwrap_or(Form::Opaque);
        let named = |inner: Vec<Form>| Form::Named(parent.to_string(), inner);
        let output = match (kind, last) {
            (Family::Carrier | Family::Mutex | Family::Init, "new" | "from") => named(vec![first]),
            (Family::Semaphore | Family::Options | Family::Atomic, "new") => named(Vec::new()),
            (Family::File, "open" | "create" | "create_new") => {
                Form::Named("std::io::Result".into(), vec![named(Vec::new())])
            }
            (Family::File, "options") => {
                Form::Named(parent.replace("File", "OpenOptions"), Vec::new())
            }
            _ => return self.dispatched(site, parent, first),
        };
        Some(output)
    }

    fn dispatched(&mut self, site: &Site, parent: &str, first: Form) -> Option<Form> {
        let actual = match first {
            Form::Opaque => Form::Named(parent.to_string(), vec![Form::Opaque]),
            other => other,
        };
        actual
            .chain()
            .into_iter()
            .find_map(|form| self.dispatch(site, &form, &actual))
    }

    pub fn radicula(&mut self, site: &Site, path: &str, shape: (&Form, &Form)) -> Form {
        let name = site.name.as_str();
        let call = if path.ends_with(&format!("::{name}")) {
            path.to_string()
        } else {
            format!("{path}::{name}")
        };
        let modeled = self
            .model
            .functions
            .get(&call)
            .map(|function| function.output.clone());
        let binding = name == "file" && call.ends_with("::Factory::file");
        let lifecycle = if binding {
            Some(&"gate")
        } else {
            RADICULA
                .iter()
                .find(|(known, _)| *known == name)
                .map(|(_, lifecycle)| lifecycle)
        };
        let Some(lifecycle) = lifecycle else {
            self.gap(
                site.span,
                format!("unmodeled Radicula call {call}; its lock lifecycle is unknown"),
            );
            return modeled.unwrap_or(Form::Opaque);
        };
        let hold = match *lifecycle {
            "manual" => Some("manual"),
            "permit" => Some("temporary"),
            "scoped" => Some("scoped"),
            _ => None,
        };
        let freestanding = shape
            .0
            .path()
            .is_some_and(|parent| parent == "radicula_lock");
        let lock = if freestanding { &Form::Opaque } else { shape.0 };
        let receiver = if freestanding || site.receiver.is_some() {
            shape.1
        } else {
            shape.0
        };
        let index = self.emit(
            site,
            (lock, receiver),
            detail("radicula", lifecycle, call, hold),
        );
        if *lifecycle == "gate" && name == "gate" {
            let key = site.args.first().map(|arg| self.text(arg.span()));
            if let Some(index) = index {
                self.harvest.findings[index].domain.key = key;
            }
        }
        modeled.unwrap_or_else(|| synthesized(name))
    }
}

fn synthesized(name: &str) -> Form {
    let result = |path: &str| {
        Form::Named(
            "std::result::Result".into(),
            vec![Form::Named(path.into(), Vec::new())],
        )
    };
    match name {
        "file" => result("radicula_lock::file::Gate"),
        "gate" | "shared" => result("radicula_lock::Gate"),
        "acquire" => result("radicula_lock::Permit"),
        "wait" | "tokio" | "asynchronous" => Form::Named("radicula_lock::Gate".into(), Vec::new()),
        "memory" | "files" => Form::Named("radicula_lock::Factory".into(), Vec::new()),
        "redis" | "connect" => result("radicula_lock::redis::Factory"),
        _ => Form::Opaque,
    }
}

fn wrapped(structs: &BTreeMap<String, Fields>, path: &str, args: Vec<Form>) -> Form {
    if let Some(outcome) = path
        .strip_suffix("::Some")
        .or_else(|| path.strip_suffix("::Ok"))
    {
        return Form::Named(outcome.into(), args.into_iter().take(1).collect());
    }
    match path.rsplit_once("::") {
        Some((parent, "default" | "new")) if structs.contains_key(parent) => {
            Form::Named(parent.into(), Vec::new())
        }
        _ => Form::Opaque,
    }
}
