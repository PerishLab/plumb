use super::Walker;
use syn::spanned::Spanned;
use syn::visit::Visit;

pub(super) struct Scan<'n> {
    pub(super) name: &'n str,
    pub(super) awaits: bool,
    pub(super) dropped: Option<usize>,
}

impl Scan<'_> {
    pub(super) fn wait(&mut self) {
        if self.dropped.is_none() {
            self.awaits = true;
        }
    }

    pub(super) fn call(&mut self, node: &syn::ExprCall) {
        let dropping = matches!(&*node.func, syn::Expr::Path(path) if path.path.is_ident("drop"));
        let named = node
            .args
            .iter()
            .any(|arg| matches!(arg, syn::Expr::Path(path) if path.path.is_ident(self.name)));
        if dropping && named && self.dropped.is_none() {
            self.dropped = Some(node.span().start().line);
        }
    }
}

impl Walker<'_> {
    pub fn held(&mut self, pat: &syn::Pat, init: &syn::Expr, rest: &[syn::Stmt]) {
        let (spine, _) = self.spine(init);
        let at = spine as *const syn::Expr as usize;
        let Some(index) = self.marks.get(&at).copied() else {
            return;
        };
        let Some(hold) = self.harvest.findings[index].hold.as_mut() else {
            return;
        };
        if hold.form != "temporary" {
            return;
        }
        let name = match pat {
            syn::Pat::Wild(_) => {
                hold.form = "discarded";
                return;
            }
            syn::Pat::Ident(ident) => ident.ident.to_string(),
            syn::Pat::Type(typed) => match &*typed.pat {
                syn::Pat::Ident(ident) => ident.ident.to_string(),
                _ => return,
            },
            _ => return,
        };
        hold.form = "binding";
        hold.name = Some(name.clone());
        let mut scan = Scan {
            name: &name,
            awaits: false,
            dropped: None,
        };
        for stmt in rest {
            scan.visit_stmt(stmt);
            if scan.dropped.is_some() {
                break;
            }
        }
        hold.across = scan.awaits;
        hold.release = scan.dropped;
    }

    pub fn awaited(&mut self, base: &syn::Expr) {
        let (spine, _) = self.spine(base);
        let at = spine as *const syn::Expr as usize;
        let Some(index) = self.marks.get(&at).copied() else {
            return;
        };
        if let Some(hold) = self.harvest.findings[index].hold.as_mut() {
            hold.awaited = true;
        }
    }
}
