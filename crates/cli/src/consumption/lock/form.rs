#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Form {
    Named(String, Vec<Form>),
    Unit,
    Param(String),
    Opaque,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    File,
    Options,
    Mutex,
    Guard,
    Stdio,
    Init,
    Atomic,
    Carrier,
    Semaphore,
    Radicula,
    Descriptor,
    Outcome,
}

const FAMILIES: [(&str, Family); 37] = [
    ("std::fs::File", Family::File),
    ("tokio::fs::File", Family::File),
    ("std::fs::OpenOptions", Family::Options),
    ("tokio::fs::OpenOptions", Family::Options),
    ("std::sync::Mutex", Family::Mutex),
    ("std::sync::RwLock", Family::Mutex),
    ("parking_lot::Mutex", Family::Mutex),
    ("parking_lot::RwLock", Family::Mutex),
    ("parking_lot::ReentrantMutex", Family::Mutex),
    ("tokio::sync::Mutex", Family::Mutex),
    ("tokio::sync::RwLock", Family::Mutex),
    ("futures::lock::Mutex", Family::Mutex),
    ("async_lock::Mutex", Family::Mutex),
    ("async_lock::RwLock", Family::Mutex),
    ("spin::Mutex", Family::Mutex),
    ("spin::RwLock", Family::Mutex),
    ("std::io::Stdin", Family::Stdio),
    ("std::io::Stdout", Family::Stdio),
    ("std::io::Stderr", Family::Stdio),
    ("std::sync::OnceLock", Family::Init),
    ("std::sync::LazyLock", Family::Init),
    ("std::sync::Once", Family::Init),
    ("std::cell::OnceCell", Family::Init),
    ("std::cell::LazyCell", Family::Init),
    ("once_cell::sync::Lazy", Family::Init),
    ("once_cell::sync::OnceCell", Family::Init),
    ("std::sync::Arc", Family::Carrier),
    ("std::rc::Rc", Family::Carrier),
    ("std::boxed::Box", Family::Carrier),
    ("tokio::sync::Semaphore", Family::Semaphore),
    ("fd_lock::RwLock", Family::Descriptor),
    ("std::result::Result", Family::Outcome),
    ("std::option::Option", Family::Outcome),
    ("std::sync::LockResult", Family::Outcome),
    ("std::sync::TryLockResult", Family::Outcome),
    ("std::io::Result", Family::Outcome),
    ("anyhow::Result", Family::Outcome),
];

pub const PRELUDE: [(&str, &str); 12] = [
    ("Box", "std::boxed::Box"),
    ("Vec", "std::vec::Vec"),
    ("String", "std::string::String"),
    ("Option", "std::option::Option"),
    ("Some", "std::option::Option::Some"),
    ("None", "std::option::Option::None"),
    ("Result", "std::result::Result"),
    ("Ok", "std::result::Result::Ok"),
    ("Err", "std::result::Result::Err"),
    ("drop", "std::mem::drop"),
    ("Default", "std::default::Default"),
    ("ToString", "std::string::ToString"),
];

pub fn family(path: &str) -> Option<Family> {
    if let Some((_, found)) = FAMILIES.iter().find(|(name, _)| *name == path) {
        return Some(*found);
    }
    if path.starts_with("radicula_lock::") {
        return Some(Family::Radicula);
    }
    if path.starts_with("std::sync::atomic::Atomic") {
        return Some(Family::Atomic);
    }
    if path.ends_with("Guard") {
        return Some(Family::Guard);
    }
    None
}

pub fn canonical(path: String) -> String {
    for prefix in ["core::", "alloc::"] {
        if let Some(rest) = path.strip_prefix(prefix) {
            return format!("std::{rest}");
        }
    }
    path
}

impl Form {
    pub fn family(&self) -> Option<Family> {
        match self {
            Self::Named(path, _) => family(path),
            _ => None,
        }
    }

    pub fn path(&self) -> Option<&str> {
        match self {
            Self::Named(path, _) => Some(path),
            _ => None,
        }
    }

    pub fn first(&self) -> Form {
        match self {
            Self::Named(_, args) => args.first().cloned().unwrap_or(Self::Opaque),
            _ => Self::Opaque,
        }
    }

    pub fn peel(&self) -> Form {
        if self.family() == Some(Family::Outcome) {
            return self.first().peel();
        }
        self.clone()
    }

    pub fn inner(&self) -> Option<Form> {
        match self.family() {
            Some(Family::Carrier | Family::Guard | Family::Init) => Some(self.first()),
            _ => None,
        }
    }

    pub fn chain(&self) -> Vec<Form> {
        let mut forms = vec![self.peel()];
        while let Some(next) = forms.last().and_then(Form::inner) {
            if forms.len() > 8 {
                break;
            }
            forms.push(next.peel());
        }
        forms
    }

    pub fn substitute(&self, params: &[String], args: &[Form]) -> Form {
        match self {
            Self::Param(name) => params
                .iter()
                .position(|param| param == name)
                .and_then(|index| args.get(index).cloned())
                .unwrap_or(Self::Opaque),
            Self::Named(path, inner) => Self::Named(
                path.clone(),
                inner
                    .iter()
                    .map(|form| form.substitute(params, args))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    pub fn render(&self) -> String {
        match self {
            Self::Named(path, args) if args.is_empty() => path.clone(),
            Self::Named(path, args) => {
                let inner: Vec<String> = args.iter().map(Form::render).collect();
                format!("{path}<{}>", inner.join(", "))
            }
            Self::Unit => "()".into(),
            Self::Param(name) => name.clone(),
            Self::Opaque => "_".into(),
        }
    }
}
