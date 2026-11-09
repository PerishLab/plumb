use crate::shape::web::Syntax;
use std::collections::BTreeMap;
use tree_sitter::Node;

pub(super) struct Scope<'a>(
    pub(super) Syntax<'a>,
    pub(super) &'a BTreeMap<String, String>,
);

impl Scope<'_> {
    pub(super) fn component(&self, name: &str) -> Option<&str> {
        let component = self.1.get(name)?.strip_prefix("@perishlab/design#")?;
        ["Hero", "Navigator"]
            .contains(&component)
            .then_some(component)
    }

    pub(super) fn verify(&self, node: Node<'_>) -> Result<(), String> {
        if node.kind() == "snippet_start" {
            self.snippet(node)?;
        }
        if !self.bears(node) {
            return Ok(());
        }
        if ["each_statement", "snippet_statement", "await_statement"].contains(&node.kind()) {
            return Err("brand references inside template lexical scopes are unqualified".into());
        }
        if self
            .0
            .children(node)
            .iter()
            .any(|child| child.kind() == "const_tag")
        {
            return Err("const bindings in a brand-bearing template scope are unqualified".into());
        }
        if node.kind() == "element" {
            self.element(node)?;
        }
        Ok(())
    }

    fn snippet(&self, node: Node<'_>) -> Result<(), String> {
        for child in self.0.children(node) {
            if child.kind() == "snippet_name" && self.component(self.0.text(child)).is_some() {
                return Err("snippet name shadows an imported brand component".into());
            }
        }
        Ok(())
    }

    fn element(&self, node: Node<'_>) -> Result<(), String> {
        let start = self
            .0
            .children(node)
            .into_iter()
            .find(|child| child.kind() == "start_tag");
        let Some(start) = start else { return Ok(()) };
        for attribute in self.0.children(start) {
            if self.directive(attribute) {
                return Err(
                    "slot bindings in a brand-bearing template scope are unqualified".into(),
                );
            }
        }
        Ok(())
    }

    fn directive(&self, node: Node<'_>) -> bool {
        self.0.children(node).iter().any(|child| {
            child.kind() == "attribute_name" && self.0.text(*child).starts_with("let:")
        })
    }

    fn bears(&self, node: Node<'_>) -> bool {
        if ["start_tag", "self_closing_tag"].contains(&node.kind()) {
            return self.0.children(node).iter().any(|child| {
                child.kind() == "tag_name" && self.component(self.0.text(*child)).is_some()
            });
        }
        self.0.children(node).iter().any(|child| self.bears(*child))
    }
}
