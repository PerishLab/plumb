use super::crates::Target;
use super::model::{Model, Module};
use super::record::{Gap, Skip};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

pub struct Unit {
    pub module: String,
    pub file: usize,
    pub cfg: Vec<String>,
    pub context: &'static str,
    pub local: bool,
    pub items: Vec<syn::Item>,
}

pub struct Parsed {
    pub shown: String,
    pub text: String,
}

#[derive(Default)]
pub struct Forest {
    pub units: Vec<Unit>,
    pub files: Vec<Parsed>,
    pub gaps: Vec<Gap>,
    pub skips: Vec<Skip>,
    pub reached: BTreeSet<String>,
}

#[derive(Clone)]
struct Place {
    module: String,
    parent: Option<String>,
    here: PathBuf,
    nest: PathBuf,
    cfg: Vec<String>,
}

impl Forest {
    pub fn plant(&mut self, model: &mut Model, target: &Target) {
        model
            .externs
            .insert(target.key.clone(), target.externs.clone());
        let package = target.base.to_string_lossy().to_string();
        model.packages.insert(target.key.clone(), package);
        let here = target
            .entry
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let place = Place {
            module: target.key.clone(),
            parent: None,
            nest: here.clone(),
            here,
            cfg: Vec::new(),
        };
        self.load(model, target, &target.entry, place);
    }

    fn load(&mut self, model: &mut Model, target: &Target, disk: &Path, place: Place) {
        let shown = target.shown(disk);
        if !self.reached.insert(shown.clone()) && target.local {
            return;
        }
        let text = match std::fs::read_to_string(disk) {
            Ok(text) => text,
            Err(error) => return self.skip(shown, format!("cannot read module: {error}")),
        };
        let parsed = match syn::parse_file(&text) {
            Ok(parsed) => parsed,
            Err(error) => return self.skip(shown, format!("cannot parse Rust: {error}")),
        };
        self.files.push(Parsed { shown, text });
        let file = self.files.len() - 1;
        self.module(model, target, (file, parsed.items), place);
    }

    fn skip(&mut self, path: String, reason: String) {
        self.skips.push(Skip { path, reason });
    }

    fn module(
        &mut self,
        model: &mut Model,
        target: &Target,
        body: (usize, Vec<syn::Item>),
        place: Place,
    ) {
        let (file, mut items) = body;
        let mut scope = Module {
            root: target.key.clone(),
            parent: place.parent.clone(),
            ..Module::default()
        };
        for item in &items {
            declare(item, &mut scope);
        }
        model.modules.insert(place.module.clone(), scope);
        for item in items.iter_mut() {
            if let syn::Item::Mod(child) = item {
                self.child(model, target, (file, child), &place);
            }
        }
        self.units.push(Unit {
            module: place.module,
            file,
            cfg: place.cfg,
            context: target.context,
            local: target.local,
            items,
        });
    }

    fn child(
        &mut self,
        model: &mut Model,
        target: &Target,
        at: (usize, &mut syn::ItemMod),
        place: &Place,
    ) {
        let (file, child) = at;
        let name = child.ident.to_string();
        let mut cfg = place.cfg.clone();
        cfg.extend(conditions(&child.attrs));
        let mut next = Place {
            module: format!("{}::{name}", place.module),
            parent: Some(place.module.clone()),
            here: place.nest.join(&name),
            nest: place.nest.join(&name),
            cfg,
        };
        if let Some((_, content)) = child.content.as_mut() {
            let items = std::mem::take(content);
            return self.module(model, target, (file, items), next);
        }
        let candidates = match pathed(&child.attrs) {
            Some(path) => vec![place.here.join(path)],
            None => vec![
                place.nest.join(format!("{name}.rs")),
                place.nest.join(&name).join("mod.rs"),
            ],
        };
        let Some(disk) = candidates
            .into_iter()
            .map(|path| clean(&path))
            .find(|path| path.is_file())
        else {
            let start = child.ident.span().start();
            return self.gaps.push(Gap {
                path: self.files[file].shown.clone(),
                line: start.line,
                column: start.column + 1,
                item: next.module,
                reason: "module file absent".into(),
                text: format!("mod {name};"),
            });
        };
        next.here = disk.parent().map(Path::to_path_buf).unwrap_or_default();
        next.nest = if disk.ends_with("mod.rs") || pathed(&child.attrs).is_some() {
            next.here.clone()
        } else {
            disk.with_extension("")
        };
        self.load(model, target, &disk, next);
    }
}

fn declare(item: &syn::Item, scope: &mut Module) {
    let name = match item {
        syn::Item::Use(used) => return flatten(&used.tree, Vec::new(), scope),
        syn::Item::Struct(item) => &item.ident,
        syn::Item::Enum(item) => &item.ident,
        syn::Item::Union(item) => &item.ident,
        syn::Item::Type(item) => &item.ident,
        syn::Item::Fn(item) => &item.sig.ident,
        syn::Item::Const(item) => &item.ident,
        syn::Item::Static(item) => &item.ident,
        syn::Item::Trait(item) => &item.ident,
        syn::Item::Mod(item) => &item.ident,
        syn::Item::ExternCrate(item) => {
            let alias = item
                .rename
                .as_ref()
                .map_or(&item.ident, |(_, rename)| rename);
            scope
                .names
                .insert(alias.to_string(), vec![item.ident.to_string()]);
            return;
        }
        syn::Item::Macro(item) => match &item.ident {
            Some(ident) => ident,
            None => return,
        },
        _ => return,
    };
    scope.items.insert(name.to_string());
}

fn flatten(tree: &syn::UseTree, prefix: Vec<String>, scope: &mut Module) {
    let extend = |name: &syn::Ident| {
        let mut path = prefix.clone();
        if name != "self" {
            path.push(name.to_string());
        }
        path
    };
    match tree {
        syn::UseTree::Path(path) => flatten(&path.tree, extend(&path.ident), scope),
        syn::UseTree::Name(name) => {
            let path = extend(&name.ident);
            if let Some(alias) = path.last().cloned() {
                scope.names.insert(alias, path);
            }
        }
        syn::UseTree::Rename(rename) if rename.rename == "_" => {
            scope.hidden.push(extend(&rename.ident))
        }
        syn::UseTree::Rename(rename) => {
            scope
                .names
                .insert(rename.rename.to_string(), extend(&rename.ident));
        }
        syn::UseTree::Glob(_) => scope.globs.push(prefix),
        syn::UseTree::Group(group) => {
            for tree in &group.items {
                flatten(tree, prefix.clone(), scope);
            }
        }
    }
}

pub fn conditions(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .filter_map(|attr| attr.meta.require_list().ok())
        .map(|list| tidy(&list.tokens.to_string()))
        .collect()
}

pub fn tidy(text: &str) -> String {
    let mut text = text.to_string();
    for (from, to) in [
        (" ,", ","),
        ("( ", "("),
        (" )", ")"),
        (" (", "("),
        ("! ", "!"),
        (" ::", "::"),
        (":: ", "::"),
    ] {
        text = text.replace(from, to);
    }
    text
}

fn pathed(attrs: &[syn::Attribute]) -> Option<String> {
    attrs
        .iter()
        .find(|attr| attr.path().is_ident("path"))
        .and_then(|attr| {
            let syn::Meta::NameValue(pair) = &attr.meta else {
                return None;
            };
            match &pair.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(text),
                    ..
                }) => Some(text.value()),
                _ => None,
            }
        })
}

fn clean(path: &Path) -> PathBuf {
    let mut parts = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop();
            }
            other => parts.push(other.as_os_str()),
        }
    }
    parts
}
