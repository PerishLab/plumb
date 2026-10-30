use super::emit::{core, transparent};
use super::{Binding, Walker};
use crate::consumption::lock::form::{Family, Form};
use std::collections::BTreeMap;
use syn::spanned::Spanned;

impl Walker<'_> {
    pub fn infer(&mut self, expr: &syn::Expr) -> Form {
        let at = expr as *const syn::Expr as usize;
        match expr {
            syn::Expr::Path(path) => self.path(path, at),
            syn::Expr::Field(field) => {
                let base = self.infer(&field.base);
                self.field(&base, &field.member, at)
            }
            syn::Expr::Call(call) => self.call(call, at),
            syn::Expr::MethodCall(call) => self.method(call, at),
            syn::Expr::Try(inner) => self.infer(&inner.expr).peel(),
            syn::Expr::Await(inner) => {
                let form = self.infer(&inner.base);
                self.awaited(&inner.base);
                form
            }
            syn::Expr::Paren(inner) => self.infer(&inner.expr),
            syn::Expr::Group(inner) => self.infer(&inner.expr),
            syn::Expr::Reference(inner) => self.infer(&inner.expr),
            syn::Expr::Unary(inner) => {
                let form = self.infer(&inner.expr);
                match inner.op {
                    syn::UnOp::Deref(_) => form.peel().inner().unwrap_or(form),
                    _ => Form::Opaque,
                }
            }
            syn::Expr::Tuple(tuple) if tuple.elems.is_empty() => Form::Unit,
            syn::Expr::Struct(structure) => self.structure(structure),
            syn::Expr::Macro(mac) => self.mac(&mac.mac),
            syn::Expr::Block(block) => self.block(&block.block),
            syn::Expr::Unsafe(block) => self.block(&block.block),
            syn::Expr::Async(block) => {
                self.block(&block.block);
                Form::Opaque
            }
            syn::Expr::Closure(closure) => self.closure(closure),
            syn::Expr::Let(binding) => {
                let form = self.infer(&binding.expr);
                self.bind(&binding.pat, form, Some(self.text(binding.expr.span())));
                Form::Opaque
            }
            syn::Expr::If(branch) => self.branch(branch),
            syn::Expr::Match(choice) => self.choose(choice),
            syn::Expr::ForLoop(each) => {
                self.infer(&each.expr);
                self.frames.push(BTreeMap::new());
                self.bind(&each.pat, Form::Opaque, None);
                self.block(&each.body);
                self.frames.pop();
                Form::Opaque
            }
            syn::Expr::While(each) => {
                self.frames.push(BTreeMap::new());
                self.infer(&each.cond);
                self.block(&each.body);
                self.frames.pop();
                Form::Opaque
            }
            _ => {
                syn::visit::visit_expr(self, expr);
                Form::Opaque
            }
        }
    }

    fn path(&mut self, path: &syn::ExprPath, at: usize) -> Form {
        if let Some(ident) = path.path.get_ident()
            && let Some(binding) = self.lookup(&ident.to_string())
        {
            return binding.form.clone();
        }
        let Some(resolved) = self.resolve(&path.path) else {
            return Form::Opaque;
        };
        if let Some(form) = self.model.statics.get(&resolved) {
            self.owners.insert(at, resolved);
            return form.clone();
        }
        if self.model.structs.contains_key(&resolved) {
            return Form::Named(resolved, Vec::new());
        }
        Form::Opaque
    }

    pub fn resolve(&self, path: &syn::Path) -> Option<String> {
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect();
        if segments.first().is_some_and(|head| head == "Self") {
            let owner = self.scope.owner.as_ref()?.path()?.to_string();
            let rest: Vec<&str> = segments[1..].iter().map(String::as_str).collect();
            return Some(
                std::iter::once(owner.as_str())
                    .chain(rest)
                    .collect::<Vec<_>>()
                    .join("::"),
            );
        }
        self.model.resolve(&self.scope.module, &segments)
    }

    fn field(&mut self, base: &Form, member: &syn::Member, at: usize) -> Form {
        let name = match member {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(index) => index.index.to_string(),
        };
        for form in base.chain() {
            let Some(path) = form.path() else {
                continue;
            };
            if let Some(found) = self
                .model
                .structs
                .get(path)
                .and_then(|fields| fields.get(&name))
            {
                self.owners.insert(at, format!("{path}::{name}"));
                return found.clone();
            }
        }
        Form::Opaque
    }

    fn structure(&mut self, structure: &syn::ExprStruct) -> Form {
        for field in &structure.fields {
            self.infer(&field.expr);
        }
        if let Some(rest) = &structure.rest {
            self.infer(rest);
        }
        match self.resolve(&structure.path) {
            Some(path) => Form::Named(path, Vec::new()),
            None => Form::Opaque,
        }
    }

    fn closure(&mut self, closure: &syn::ExprClosure) -> Form {
        self.frames.push(BTreeMap::new());
        for input in &closure.inputs {
            self.bind(input, Form::Opaque, Some("parameter".into()));
        }
        self.infer(&closure.body);
        self.frames.pop();
        Form::Opaque
    }

    fn branch(&mut self, branch: &syn::ExprIf) -> Form {
        self.frames.push(BTreeMap::new());
        self.infer(&branch.cond);
        let form = self.block(&branch.then_branch);
        self.frames.pop();
        let Some((_, other)) = &branch.else_branch else {
            return form;
        };
        let alternative = self.infer(other);
        if matches!(form, Form::Opaque) {
            alternative
        } else {
            form
        }
    }

    fn choose(&mut self, choice: &syn::ExprMatch) -> Form {
        let scrutinee = self.infer(&choice.expr);
        let mut form = Form::Opaque;
        for arm in &choice.arms {
            self.frames.push(BTreeMap::new());
            self.bind(&arm.pat, scrutinee.clone(), None);
            if let Some((_, guard)) = &arm.guard {
                self.infer(guard);
            }
            let result = self.infer(&arm.body);
            self.frames.pop();
            if matches!(form, Form::Opaque) {
                form = result;
            }
        }
        form
    }

    pub fn bind(&mut self, pat: &syn::Pat, form: Form, origin: Option<String>) {
        match pat {
            syn::Pat::Ident(ident) => {
                if let Some((_, inner)) = &ident.subpat {
                    self.bind(inner, form.clone(), origin.clone());
                }
                self.define(ident.ident.to_string(), Binding { form, origin });
            }
            syn::Pat::Type(typed) => {
                let form = self.model.form(&self.scope, &typed.ty);
                self.bind(&typed.pat, form, origin);
            }
            syn::Pat::Reference(inner) => self.bind(&inner.pat, form, origin),
            syn::Pat::Paren(inner) => self.bind(&inner.pat, form, origin),
            syn::Pat::TupleStruct(tuple) => {
                let wrapped = tuple
                    .path
                    .segments
                    .last()
                    .is_some_and(|last| last.ident == "Ok" || last.ident == "Some");
                let inner = if wrapped && form.family() == Some(Family::Outcome) {
                    form.first()
                } else {
                    Form::Opaque
                };
                for elem in &tuple.elems {
                    self.bind(elem, inner.clone(), origin.clone());
                }
            }
            syn::Pat::Tuple(tuple) => {
                for elem in &tuple.elems {
                    self.bind(elem, Form::Opaque, None);
                }
            }
            syn::Pat::Struct(structure) => {
                for field in &structure.fields {
                    self.bind(&field.pat, Form::Opaque, None);
                }
            }
            syn::Pat::Slice(slice) => {
                for elem in &slice.elems {
                    self.bind(elem, Form::Opaque, None);
                }
            }
            syn::Pat::Or(choice) => {
                for case in &choice.cases {
                    self.bind(case, form.clone(), origin.clone());
                }
            }
            _ => {}
        }
    }

    pub fn block(&mut self, block: &syn::Block) -> Form {
        self.frames.push(BTreeMap::new());
        let mut last = Form::Opaque;
        for (index, stmt) in block.stmts.iter().enumerate() {
            last = Form::Opaque;
            match stmt {
                syn::Stmt::Local(local) => self.local(local, &block.stmts[index + 1..]),
                syn::Stmt::Item(item) => self.visit(item),
                syn::Stmt::Expr(expr, None) => last = self.infer(expr),
                syn::Stmt::Expr(expr, Some(_)) => {
                    self.infer(expr);
                }
                syn::Stmt::Macro(mac) => {
                    self.mac(&mac.mac);
                }
            }
        }
        self.frames.pop();
        last
    }

    fn local(&mut self, local: &syn::Local, rest: &[syn::Stmt]) {
        let Some(init) = &local.init else {
            return self.bind(&local.pat, Form::Opaque, None);
        };
        let form = self.infer(&init.expr);
        if let Some((_, other)) = &init.diverge {
            self.infer(other);
        }
        self.held(&local.pat, &init.expr, rest);
        self.bind(&local.pat, form, Some(self.text(init.expr.span())));
    }

    pub fn spine<'e>(&self, expr: &'e syn::Expr) -> (&'e syn::Expr, bool) {
        match expr {
            syn::Expr::Try(inner) => self.spine(&inner.expr),
            syn::Expr::Paren(inner) => self.spine(&inner.expr),
            syn::Expr::Await(inner) => (self.spine(&inner.base).0, true),
            syn::Expr::MethodCall(call) if peeled(&call.method.to_string()) => {
                self.spine(&call.receiver)
            }
            _ => (core(expr), false),
        }
    }
}

pub fn peeled(name: &str) -> bool {
    transparent(name)
        || matches!(
            name,
            "unwrap" | "expect" | "map_err" | "context" | "with_context" | "ok" | "unwrap_or_else"
        )
}
