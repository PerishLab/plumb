use super::emit::transparent;
use super::infer::peeled;
use super::{Edge, Walker};
use crate::consumption::lock::form::{Family, Form};
use proc_macro2::Span;
use syn::spanned::Spanned;

pub const SUSPECT: [&str; 12] = [
    "lock",
    "try_lock",
    "lock_shared",
    "try_lock_shared",
    "unlock",
    "lock_exclusive",
    "try_lock_exclusive",
    "blocking_lock",
    "lock_owned",
    "try_lock_owned",
    "blocking_read",
    "blocking_write",
];

const ACQUIRE: [&str; 17] = [
    "lock",
    "try_lock",
    "read",
    "write",
    "try_read",
    "try_write",
    "blocking_lock",
    "blocking_read",
    "blocking_write",
    "lock_owned",
    "try_lock_owned",
    "read_owned",
    "write_owned",
    "lock_arc",
    "read_arc",
    "write_arc",
    "upgradable_read",
];

const INHERENT: [&str; 5] = [
    "lock",
    "lock_shared",
    "try_lock",
    "try_lock_shared",
    "unlock",
];

pub struct Site<'s> {
    pub span: Span,
    pub at: usize,
    pub receiver: Option<&'s syn::Expr>,
    pub args: Vec<&'s syn::Expr>,
    pub name: String,
}

pub struct Detail {
    pub role: &'static str,
    pub lifecycle: Option<&'static str>,
    pub call: String,
    pub hold: Option<&'static str>,
}

impl Walker<'_> {
    pub fn method(&mut self, call: &syn::ExprMethodCall, at: usize) -> Form {
        let receiver = self.infer(&call.receiver);
        for arg in &call.args {
            self.infer(arg);
        }
        let name = call.method.to_string();
        if transparent(&name) || matches!(name.as_str(), "map_err" | "context" | "with_context") {
            return receiver;
        }
        if peeled(&name) {
            return receiver.peel();
        }
        let site = Site {
            span: call.method.span(),
            at,
            receiver: Some(&call.receiver),
            args: call.args.iter().collect(),
            name: name.clone(),
        };
        for form in receiver.chain() {
            if let Some(output) = self.dispatch(&site, &form, &receiver) {
                return output;
            }
        }
        if SUSPECT.contains(&name.as_str()) {
            let reason = match receiver.peel() {
                Form::Named(path, _) => {
                    format!("lock-named method `{name}` on unmodeled type {path}")
                }
                _ => format!("lock-named method `{name}` on a receiver whose type is not inferred"),
            };
            self.gap(call.span(), reason);
        }
        Form::Opaque
    }

    pub fn dispatch(&mut self, site: &Site, form: &Form, whole: &Form) -> Option<Form> {
        let name = site.name.as_str();
        let path = form.path().unwrap_or_default().to_string();
        match form.family() {
            Some(Family::Mutex) if ACQUIRE.contains(&name) => {
                let role = self.payload(site.span, form);
                self.emit(
                    site,
                    (form, whole),
                    detail(
                        role,
                        "acquire",
                        format!("{path}::{name}"),
                        Some("temporary"),
                    ),
                );
                let guard = Form::Named(format!("{path}Guard"), vec![form.first()]);
                Some(Form::Named("std::result::Result".into(), vec![guard]))
            }
            Some(Family::Mutex) if name == "get_mut" => Some(form.first()),
            Some(Family::Semaphore) if name.contains("acquire") => {
                self.emit(
                    site,
                    (form, whole),
                    detail(
                        "zone",
                        "acquire",
                        format!("{path}::{name}"),
                        Some("temporary"),
                    ),
                );
                Some(Form::Opaque)
            }
            Some(Family::File) => Some(self.file(site, (form, whole))),
            Some(Family::Options) if name == "open" => {
                let file = path.replace("OpenOptions", "File");
                Some(Form::Named(
                    "std::io::Result".into(),
                    vec![Form::Named(file, Vec::new())],
                ))
            }
            Some(Family::Options) => Some(form.clone()),
            Some(Family::Stdio) if name == "lock" => {
                self.emit(
                    site,
                    (form, whole),
                    detail("stdio", "acquire", format!("{path}::{name}"), None),
                );
                Some(Form::Opaque)
            }
            Some(Family::Descriptor)
                if matches!(name, "read" | "write" | "try_read" | "try_write") =>
            {
                self.emit(
                    site,
                    (form, whole),
                    detail(
                        "primitive",
                        "acquire",
                        format!("{path}::{name}"),
                        Some("temporary"),
                    ),
                );
                Some(Form::Opaque)
            }
            Some(Family::Radicula) => Some(self.radicula(site, &path, (form, whole))),
            Some(Family::Init) => match name {
                "get_or_init" | "force" => Some(form.first()),
                "get" => Some(Form::Named(
                    "std::option::Option".into(),
                    vec![form.first()],
                )),
                _ => None,
            },
            Some(Family::Carrier | Family::Guard | Family::Outcome) => None,
            Some(_) => Some(Form::Opaque),
            None => self.modeled(site, &path),
        }
    }

    fn file(&mut self, site: &Site, shape: (&Form, &Form)) -> Form {
        let name = site.name.as_str();
        let path = shape.0.path().unwrap_or_default().to_string();
        let inherent = path == "std::fs::File" && INHERENT.contains(&name);
        if inherent || SUSPECT.contains(&name) {
            let call = if inherent {
                format!("{path}::{name}")
            } else {
                let traits = self.model.traits(&self.scope.module);
                let found = traits
                    .into_iter()
                    .find(|candidate| candidate.ends_with("FileExt"));
                format!("{}::{name}", found.unwrap_or_else(|| "FileExt".into()))
            };
            let releasing = name == "unlock";
            let lifecycle = if releasing { "release" } else { "acquire" };
            let hold = (!releasing).then_some("handle");
            self.emit(site, shape, detail("primitive", lifecycle, call, hold));
            return Form::Opaque;
        }
        match name {
            "try_clone" => Form::Named("std::io::Result".into(), vec![shape.0.clone()]),
            "into_std" => Form::Named("std::fs::File".into(), Vec::new()),
            _ => Form::Opaque,
        }
    }

    fn modeled(&mut self, site: &Site, path: &str) -> Option<Form> {
        if path.is_empty() {
            return None;
        }
        let callee = format!("{path}::{}", site.name);
        let output = self.model.functions.get(&callee)?.output.clone();
        self.edge(site, callee);
        Some(output)
    }

    pub fn edge(&mut self, site: &Site, callee: String) {
        let mut finding = None;
        if self.unit.local {
            let mut record = self.finding(site.span, "call", "indirect");
            record.call = Some(callee.clone());
            record.hold = Some(self.hold("temporary"));
            if let Some(receiver) = site.receiver {
                record.domain = self.domain(receiver);
            }
            finding = self.push(record, Some(site.at));
        }
        self.harvest.edges.push(Edge {
            caller: self.item.clone(),
            callee,
            owner: self.package.clone(),
            finding,
        });
    }

    pub fn emit(&mut self, site: &Site, shape: (&Form, &Form), detail: Detail) -> Option<usize> {
        let (lock, whole) = shape;
        let mut finding = self.finding(site.span, "call", detail.role);
        finding.lifecycle = detail.lifecycle;
        finding.call = Some(detail.call);
        if !matches!(lock, Form::Opaque) {
            finding.kind = Some(lock.render());
        }
        let carrier = whole.peel();
        if carrier != *lock && !matches!(carrier, Form::Opaque) {
            finding.carrier = Some(carrier.render());
        }
        finding.hold = detail.hold.map(|form| self.hold(form));
        if let Some(receiver) = site.receiver {
            finding.domain = self.domain(receiver);
        }
        if let Some(hold) = finding.hold.as_mut()
            && hold.form == "handle"
        {
            hold.name.clone_from(&finding.domain.receiver);
        }
        self.push(finding, Some(site.at))
    }
}

pub fn detail(
    role: &'static str,
    lifecycle: &'static str,
    call: String,
    hold: Option<&'static str>,
) -> Detail {
    Detail {
        role,
        lifecycle: Some(lifecycle),
        call,
        hold,
    }
}
