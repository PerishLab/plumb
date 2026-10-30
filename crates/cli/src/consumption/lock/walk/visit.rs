use super::Walker;
use super::hold::Scan;
use syn::visit::Visit;

impl<'ast> Visit<'ast> for Walker<'_> {
    fn visit_expr(&mut self, expr: &'ast syn::Expr) {
        self.infer(expr);
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.block(block);
    }

    fn visit_item(&mut self, item: &'ast syn::Item) {
        self.visit(item);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.mac(mac);
    }
}

impl<'ast> Visit<'ast> for Scan<'_> {
    fn visit_expr_await(&mut self, node: &'ast syn::ExprAwait) {
        self.wait();
        syn::visit::visit_expr_await(self, node);
    }

    fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}

    fn visit_expr_async(&mut self, _: &'ast syn::ExprAsync) {}

    fn visit_item(&mut self, _: &'ast syn::Item) {}

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        self.call(node);
        syn::visit::visit_expr_call(self, node);
    }
}
