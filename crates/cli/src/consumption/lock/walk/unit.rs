use super::{Binding, Walker};
use crate::consumption::lock::form::Form;
use crate::consumption::lock::items::generics;
use crate::consumption::lock::tree::conditions;
use std::collections::BTreeMap;
use syn::spanned::Spanned;

struct Saved {
    item: String,
    cfg: Vec<String>,
    context: &'static str,
    frames: Vec<BTreeMap<String, Binding>>,
    generics: Vec<String>,
    owner: Option<Form>,
}

impl Walker<'_> {
    pub fn visit(&mut self, item: &syn::Item) {
        match item {
            syn::Item::Fn(item) => {
                let path = format!("{}::{}", self.unit.module, item.sig.ident);
                self.function(&item.attrs, &item.sig, &item.block, path);
            }
            syn::Item::Impl(item) => self.implement(item),
            syn::Item::Trait(item) => self.declare(item),
            syn::Item::Struct(item) => self.fields(item),
            syn::Item::Static(item) => {
                let path = format!("{}::{}", self.unit.module, item.ident);
                self.constant((&item.attrs, &item.ty, &item.expr), (path, "static"));
            }
            syn::Item::Const(item) => {
                let path = format!("{}::{}", self.unit.module, item.ident);
                self.constant((&item.attrs, &item.ty, &item.expr), (path, "static"));
            }
            syn::Item::Type(item) => {
                let path = format!("{}::{}", self.unit.module, item.ident);
                let raw = self.model.shape(&self.scope, &item.ty);
                let form = self.model.expand(raw, 0);
                self.typed(item.ident.span(), (&form, Vec::new()), (path, "alias"));
            }
            syn::Item::Macro(item) => {
                self.mac(&item.mac);
            }
            _ => {}
        }
    }

    fn save(&mut self) -> Saved {
        Saved {
            item: self.item.clone(),
            cfg: self.cfg.clone(),
            context: self.context,
            frames: std::mem::take(&mut self.frames),
            generics: self.scope.generics.clone(),
            owner: self.scope.owner.clone(),
        }
    }

    fn restore(&mut self, saved: Saved) {
        self.item = saved.item;
        self.cfg = saved.cfg;
        self.context = saved.context;
        self.frames = saved.frames;
        self.scope.generics = saved.generics;
        self.scope.owner = saved.owner;
    }

    pub fn function(
        &mut self,
        attrs: &[syn::Attribute],
        sig: &syn::Signature,
        block: &syn::Block,
        path: String,
    ) {
        let saved = self.save();
        self.item = path;
        self.cfg.extend(conditions(attrs));
        if attrs.iter().any(tested) {
            self.context = "test";
        }
        self.scope.generics.extend(generics(&sig.generics));
        self.attributes(attrs);
        self.frames.push(BTreeMap::new());
        for input in &sig.inputs {
            match input {
                syn::FnArg::Receiver(_) => {
                    let form = self.scope.owner.clone().unwrap_or(Form::Opaque);
                    let origin = Some("self".to_string());
                    self.define("self".into(), Binding { form, origin });
                }
                syn::FnArg::Typed(typed) => {
                    let form = self.model.form(&self.scope, &typed.ty);
                    self.bind(&typed.pat, form, Some("parameter".into()));
                }
            }
        }
        self.block(block);
        self.restore(saved);
    }

    fn implement(&mut self, item: &syn::ItemImpl) {
        let saved = self.save();
        self.cfg.extend(conditions(&item.attrs));
        self.scope.generics = generics(&item.generics);
        let owner = self.model.form(&self.scope, &item.self_ty);
        let base = owner.path().unwrap_or("_").to_string();
        self.scope.owner = Some(owner);
        for member in &item.items {
            match member {
                syn::ImplItem::Fn(method) => {
                    let path = format!("{base}::{}", method.sig.ident);
                    self.function(&method.attrs, &method.sig, &method.block, path);
                }
                syn::ImplItem::Const(constant) => {
                    self.item = format!("{base}::{}", constant.ident);
                    self.infer(&constant.expr);
                }
                _ => {}
            }
        }
        self.restore(saved);
    }

    fn declare(&mut self, item: &syn::ItemTrait) {
        let base = format!("{}::{}", self.unit.module, item.ident);
        for member in &item.items {
            if let syn::TraitItem::Fn(method) = member
                && let Some(block) = &method.default
            {
                let path = format!("{base}::{}", method.sig.ident);
                self.function(&method.attrs, &method.sig, block, path);
            }
        }
    }

    fn fields(&mut self, item: &syn::ItemStruct) {
        let saved = self.save();
        self.cfg.extend(conditions(&item.attrs));
        self.scope.generics = generics(&item.generics);
        let base = format!("{}::{}", self.unit.module, item.ident);
        for (index, field) in item.fields.iter().enumerate() {
            let name = field
                .ident
                .as_ref()
                .map_or(index.to_string(), ToString::to_string);
            let raw = self.model.shape(&self.scope, &field.ty);
            let via = aliased(&raw, self.model);
            let form = self.model.expand(raw, 0);
            self.item = base.clone();
            self.typed(
                field.ty.span(),
                (&form, via),
                (format!("{base}::{name}"), "field"),
            );
        }
        self.restore(saved);
    }

    fn constant(
        &mut self,
        parts: (&[syn::Attribute], &syn::Type, &syn::Expr),
        named: (String, &'static str),
    ) {
        let (attrs, ty, expr) = parts;
        let saved = self.save();
        self.cfg.extend(conditions(attrs));
        let raw = self.model.shape(&self.scope, ty);
        let via = aliased(&raw, self.model);
        let form = self.model.expand(raw, 0);
        self.item = named.0.clone();
        self.typed(ty.span(), (&form, via), named);
        self.frames.push(BTreeMap::new());
        self.infer(expr);
        self.restore(saved);
    }

    fn attributes(&mut self, attrs: &[syn::Attribute]) {
        for attr in attrs {
            let segments: Vec<String> = attr
                .path()
                .segments
                .iter()
                .map(|part| part.ident.to_string())
                .collect();
            let Some(path) = self.model.resolve(&self.scope.module, &segments) else {
                continue;
            };
            if path.starts_with("radicula_lock::") {
                let mut finding = self.finding(attr.span(), "attribute", "radicula");
                finding.lifecycle = Some("scoped");
                finding.call = Some(path);
                finding.domain.key = attr
                    .meta
                    .require_list()
                    .ok()
                    .map(|list| self.text(list.tokens.span()));
                finding.hold = Some(self.hold("scoped"));
                self.push(finding, None);
            }
        }
    }
}

fn tested(attr: &syn::Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|last| last.ident == "test")
}

fn aliased(raw: &Form, model: &crate::consumption::lock::model::Model) -> Vec<String> {
    match raw {
        Form::Named(path, args) => {
            let mut found: Vec<String> = Vec::new();
            if model.aliases.contains_key(path) {
                found.push(path.clone());
            }
            for arg in args {
                found.extend(aliased(arg, model));
            }
            found
        }
        _ => Vec::new(),
    }
}
