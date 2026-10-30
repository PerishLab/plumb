use super::form::{Form, PRELUDE, canonical};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct Module {
    pub root: String,
    pub parent: Option<String>,
    pub names: BTreeMap<String, Vec<String>>,
    pub globs: Vec<Vec<String>>,
    pub hidden: Vec<Vec<String>>,
    pub items: BTreeSet<String>,
}

pub type Table = BTreeMap<String, String>;

pub type Fields = BTreeMap<String, Form>;

type Memo = RefCell<BTreeMap<Vec<String>, Option<String>>>;

pub struct Alias {
    pub params: Vec<String>,
    pub target: Form,
}

pub struct Function {
    pub output: Form,
    pub owner: String,
}

#[derive(Default)]
pub struct Model {
    pub modules: BTreeMap<String, Module>,
    pub externs: BTreeMap<String, Table>,
    pub structs: BTreeMap<String, Fields>,
    pub aliases: BTreeMap<String, Alias>,
    pub functions: BTreeMap<String, Function>,
    pub statics: BTreeMap<String, Form>,
    pub packages: BTreeMap<String, String>,
    pub memo: Memo,
}

impl Model {
    pub fn resolve(&self, module: &str, segments: &[String]) -> Option<String> {
        self.chase(module, segments, 0).map(canonical)
    }

    fn chase(&self, module: &str, segments: &[String], depth: usize) -> Option<String> {
        let mut key = vec![module.to_string()];
        key.extend(segments.iter().cloned());
        if let Some(known) = self.memo.borrow().get(&key) {
            return known.clone();
        }
        self.memo.borrow_mut().insert(key.clone(), None);
        let found = self.trace(module, segments, depth);
        self.memo.borrow_mut().insert(key, found.clone());
        found
    }

    fn trace(&self, module: &str, segments: &[String], depth: usize) -> Option<String> {
        if depth > 24 || segments.is_empty() {
            return None;
        }
        let mut current = module.to_string();
        let mut index = 0;
        while let Some(segment) = segments.get(index) {
            match segment.as_str() {
                "self" => {}
                "super" => current = self.modules.get(&current)?.parent.clone()?,
                "crate" => current = self.modules.get(&current)?.root.clone(),
                _ => break,
            }
            index += 1;
        }
        let mut base = if index > 0 {
            current
        } else {
            index = 1;
            self.lookup(module, &segments[0], depth)?
        };
        for segment in &segments[index..] {
            base = self.step(&base, segment, depth);
        }
        Some(base)
    }

    fn lookup(&self, module: &str, name: &str, depth: usize) -> Option<String> {
        let Some(scope) = self.modules.get(module) else {
            return Some(name.to_string());
        };
        if scope.items.contains(name) {
            return Some(format!("{module}::{name}"));
        }
        if let Some(target) = scope.names.get(name) {
            return self.chase(module, target, depth + 1);
        }
        let mut guess = None;
        for glob in &scope.globs {
            let Some(target) = self.chase(module, glob, depth + 1) else {
                continue;
            };
            if let Some(found) = self.exports(&target, name, depth + 1) {
                return Some(found);
            }
            if !self.modules.contains_key(&target) && guess.is_none() {
                guess = Some(format!("{target}::{name}"));
            }
        }
        if let Some((_, path)) = PRELUDE.iter().find(|(short, _)| *short == name) {
            return Some(path.to_string());
        }
        if let Some(target) = self.externs.get(&scope.root).and_then(|map| map.get(name)) {
            return Some(target.clone());
        }
        guess.or_else(|| Some(name.to_string()))
    }

    fn defines(&self, module: &str, name: &str) -> bool {
        self.modules
            .get(module)
            .is_some_and(|scope| scope.items.contains(name) || scope.names.contains_key(name))
    }

    fn step(&self, base: &str, segment: &str, depth: usize) -> String {
        self.modules
            .get(base)
            .filter(|scope| !scope.items.contains(segment))
            .and_then(|scope| {
                let named = scope
                    .names
                    .get(segment)
                    .and_then(|target| self.chase(base, target, depth + 1));
                named.or_else(|| self.globbed(base, segment, depth))
            })
            .unwrap_or_else(|| format!("{base}::{segment}"))
    }

    fn globbed(&self, base: &str, segment: &str, depth: usize) -> Option<String> {
        let scope = self.modules.get(base)?;
        scope.globs.iter().find_map(|glob| {
            let target = self.chase(base, glob, depth + 1)?;
            self.exports(&target, segment, depth + 1)
        })
    }

    fn exports(&self, target: &str, name: &str, depth: usize) -> Option<String> {
        if self.defines(target, name) {
            return Some(self.step(target, name, depth));
        }
        if depth > 12 {
            return None;
        }
        self.globbed(target, name, depth + 1)
    }

    pub fn traits(&self, module: &str) -> Vec<String> {
        let Some(scope) = self.modules.get(module) else {
            return Vec::new();
        };
        scope
            .names
            .values()
            .chain(scope.hidden.iter())
            .filter_map(|target| self.resolve(module, target))
            .collect()
    }

    pub fn expand(&self, form: Form, depth: usize) -> Form {
        let Form::Named(path, args) = form else {
            return form;
        };
        let args: Vec<Form> = args
            .into_iter()
            .map(|arg| self.expand(arg, depth + 1))
            .collect();
        match self.aliases.get(&path) {
            Some(alias) if depth < 8 => {
                self.expand(alias.target.substitute(&alias.params, &args), depth + 1)
            }
            _ => Form::Named(path, args),
        }
    }

    pub fn form(&self, scope: &Scope, ty: &syn::Type) -> Form {
        let raw = self.shape(scope, ty);
        self.expand(raw, 0)
    }

    pub fn shape(&self, scope: &Scope, ty: &syn::Type) -> Form {
        match ty {
            syn::Type::Path(path) if path.qself.is_none() => self.named(scope, &path.path),
            syn::Type::Reference(inner) => self.shape(scope, &inner.elem),
            syn::Type::Paren(inner) => self.shape(scope, &inner.elem),
            syn::Type::Group(inner) => self.shape(scope, &inner.elem),
            syn::Type::Tuple(tuple) if tuple.elems.is_empty() => Form::Unit,
            _ => Form::Opaque,
        }
    }

    fn named(&self, scope: &Scope, path: &syn::Path) -> Form {
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect();
        if segments.len() == 1 && scope.generics.contains(&segments[0]) {
            return Form::Param(segments[0].clone());
        }
        if segments.first().is_some_and(|head| head == "Self") {
            return scope.owner.clone().unwrap_or(Form::Opaque);
        }
        let args = path
            .segments
            .last()
            .map(|last| self.arguments(scope, &last.arguments))
            .unwrap_or_default();
        match self.resolve(&scope.module, &segments) {
            Some(resolved) => Form::Named(resolved, args),
            None => Form::Opaque,
        }
    }

    fn arguments(&self, scope: &Scope, arguments: &syn::PathArguments) -> Vec<Form> {
        let syn::PathArguments::AngleBracketed(bracketed) = arguments else {
            return Vec::new();
        };
        bracketed
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(ty) => Some(self.shape(scope, ty)),
                _ => None,
            })
            .collect()
    }
}

#[derive(Clone, Default)]
pub struct Scope {
    pub module: String,
    pub owner: Option<Form>,
    pub generics: Vec<String>,
}
