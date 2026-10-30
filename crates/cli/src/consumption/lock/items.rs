use super::form::Form;
use super::model::{Alias, Function, Model, Scope};
use super::tree::Unit;
use std::collections::BTreeMap;

pub fn sign(model: &mut Model, units: &[Unit]) {
    for unit in units {
        for item in &unit.items {
            if let syn::Item::Type(alias) = item {
                let scope = Scope {
                    module: unit.module.clone(),
                    owner: None,
                    generics: generics(&alias.generics),
                };
                let target = model.shape(&scope, &alias.ty);
                let path = format!("{}::{}", unit.module, alias.ident);
                model.aliases.insert(
                    path,
                    Alias {
                        params: scope.generics,
                        target,
                    },
                );
            }
        }
    }
    for unit in units {
        for item in &unit.items {
            record(model, unit, item);
        }
    }
}

pub fn generics(generics: &syn::Generics) -> Vec<String> {
    generics
        .type_params()
        .map(|param| param.ident.to_string())
        .collect()
}

fn record(model: &mut Model, unit: &Unit, item: &syn::Item) {
    let root = model
        .modules
        .get(&unit.module)
        .and_then(|module| model.packages.get(&module.root))
        .cloned()
        .unwrap_or_default();
    let mut scope = Scope {
        module: unit.module.clone(),
        ..Scope::default()
    };
    match item {
        syn::Item::Struct(item) => {
            scope.generics = generics(&item.generics);
            let fields: BTreeMap<String, Form> = item
                .fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    let name = field
                        .ident
                        .as_ref()
                        .map_or(index.to_string(), ToString::to_string);
                    (name, model.form(&scope, &field.ty))
                })
                .collect();
            model
                .structs
                .insert(format!("{}::{}", unit.module, item.ident), fields);
        }
        syn::Item::Static(item) => {
            let form = model.form(&scope, &item.ty);
            model
                .statics
                .insert(format!("{}::{}", unit.module, item.ident), form);
        }
        syn::Item::Const(item) => {
            let form = model.form(&scope, &item.ty);
            model
                .statics
                .insert(format!("{}::{}", unit.module, item.ident), form);
        }
        syn::Item::Fn(item) => {
            scope.generics = generics(&item.sig.generics);
            let output = output(model, &scope, &item.sig.output);
            let path = format!("{}::{}", unit.module, item.sig.ident);
            model.functions.insert(
                path,
                Function {
                    output,
                    owner: root,
                },
            );
        }
        syn::Item::Impl(item) => implement(model, (scope, root), item),
        _ => {}
    }
}

fn implement(model: &mut Model, context: (Scope, String), item: &syn::ItemImpl) {
    let (mut scope, root) = context;
    scope.generics = generics(&item.generics);
    let owner = model.form(&scope, &item.self_ty);
    let Some(path) = owner.path().map(str::to_string) else {
        return;
    };
    scope.owner = Some(owner);
    for member in &item.items {
        let syn::ImplItem::Fn(method) = member else {
            continue;
        };
        let mut inner = scope.clone();
        inner.generics.extend(generics(&method.sig.generics));
        let output = output(model, &inner, &method.sig.output);
        model.functions.insert(
            format!("{path}::{}", method.sig.ident),
            Function {
                output,
                owner: root.clone(),
            },
        );
    }
}

pub fn output(model: &Model, scope: &Scope, output: &syn::ReturnType) -> Form {
    match output {
        syn::ReturnType::Default => Form::Unit,
        syn::ReturnType::Type(_, ty) => model.form(scope, ty),
    }
}
