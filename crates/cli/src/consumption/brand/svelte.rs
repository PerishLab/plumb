use crate::shape::web::Syntax;
use std::collections::BTreeMap;
use tree_sitter::{Node, Parser};

#[derive(serde::Serialize)]
pub(super) struct Mark {
    pub(super) product: String,
    pub(super) component: String,
    pub(super) surface: Option<String>,
}

pub(super) fn read(text: &str) -> Result<Vec<Mark>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_svelte_next::LANGUAGE.into())
        .map_err(|error| error.to_string())?;
    let tree = parser
        .parse(text, None)
        .ok_or("Svelte parser returned no source")?;
    if tree.root_node().has_error() {
        return Err("Svelte brand syntax is unsupported or invalid".into());
    }
    let reader = Svelte(Syntax(text));
    let mut bindings = BTreeMap::new();
    reader.scripts(tree.root_node(), &mut bindings)?;
    let mut found = Vec::new();
    reader.tags(tree.root_node(), &bindings, &mut found)?;
    Ok(found)
}

struct Svelte<'a>(Syntax<'a>);

impl Svelte<'_> {
    fn scripts(
        &self,
        root: Node<'_>,
        bindings: &mut BTreeMap<String, String>,
    ) -> Result<(), String> {
        for node in self.0.children(root) {
            if node.kind() != "script_element" {
                continue;
            }
            let start = self
                .0
                .children(node)
                .into_iter()
                .find(|node| node.kind() == "start_tag")
                .ok_or("Svelte script has no opening tag")?;
            self.scope(start)?;
            let content = self
                .0
                .children(node)
                .into_iter()
                .find(|node| node.kind() == "raw_text");
            let Some(content) = content else { continue };
            let text = self.0.text(content);
            let mut parser = Parser::new();
            parser
                .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
                .map_err(|error| error.to_string())?;
            let tree = parser
                .parse(text, None)
                .ok_or("Svelte script was not parsed")?;
            if tree.root_node().has_error() {
                return Err("Svelte script dialect is unqualified".into());
            }
            let syntax = Syntax(text);
            for item in syntax.children(tree.root_node()) {
                if item.kind() == "import_statement" {
                    syntax.import(item, bindings)?;
                }
            }
        }
        Ok(())
    }

    fn tags(
        &self,
        node: Node<'_>,
        bindings: &BTreeMap<String, String>,
        found: &mut Vec<Mark>,
    ) -> Result<(), String> {
        if [
            "each_statement",
            "snippet_statement",
            "await_statement",
            "const_tag",
        ]
        .contains(&node.kind())
        {
            return Err("template lexical bindings are outside qualified brand scope".into());
        }
        if ["start_tag", "self_closing_tag"].contains(&node.kind()) {
            self.tag(node, bindings, found)?;
        }
        if node.kind() == "script_element" || node.kind() == "style_element" {
            return Ok(());
        }
        for child in self.0.children(node) {
            self.tags(child, bindings, found)?;
        }
        Ok(())
    }

    fn scope(&self, start: Node<'_>) -> Result<(), String> {
        for node in self.0.children(start) {
            if node.kind() == "tag_name" {
                continue;
            }
            let parts = self.0.children(node);
            let name = parts
                .first()
                .map(|node| self.0.text(*node))
                .unwrap_or_default();
            if name != "lang" {
                return Err("module/context or dynamic Svelte script scope is unqualified".into());
            }
            let value = parts
                .get(1)
                .map(|node| self.0.text(*node))
                .unwrap_or_default();
            if !["\"ts\"", "'ts'", "\"js\"", "'js'"].contains(&value) {
                return Err("Svelte script dialect is unqualified".into());
            }
        }
        Ok(())
    }

    fn tag(
        &self,
        node: Node<'_>,
        bindings: &BTreeMap<String, String>,
        found: &mut Vec<Mark>,
    ) -> Result<(), String> {
        let children = self.0.children(node);
        let tag = children
            .iter()
            .find(|node| node.kind() == "tag_name")
            .ok_or("Svelte tag has no name")?;
        let Some(binding) = bindings.get(self.0.text(*tag)) else {
            return Ok(());
        };
        let Some(component) = binding.strip_prefix("@perishlab/design#") else {
            return Ok(());
        };
        if !["Navigator", "Hero"].contains(&component) {
            return Ok(());
        }
        let mut mark = None;
        let mut surface = None;
        for attribute in children.iter().filter(|node| node.kind() != "tag_name") {
            self.attribute(*attribute, &mut mark, &mut surface)?;
        }
        if let Some(product) = mark {
            found.push(Mark {
                product,
                component: component.into(),
                surface,
            });
        }
        Ok(())
    }

    fn attribute(
        &self,
        node: Node<'_>,
        mark: &mut Option<String>,
        surface: &mut Option<String>,
    ) -> Result<(), String> {
        let parts = self.0.children(node);
        let Some(name) = parts.first().filter(|node| node.kind() == "attribute_name") else {
            return Err("spread/shorthand brand component attributes are unqualified".into());
        };
        let name = self.0.text(*name);
        if name.contains(['{', '}']) {
            return Err("spread/shorthand brand component attributes are unqualified".into());
        }
        let target = match name {
            "mark" => mark,
            "surface" => surface,
            _ => return Ok(()),
        };
        if target.is_some() {
            return Err("duplicate brand component attribute".into());
        }
        let value = parts.get(1).ok_or("brand attribute has no static value")?;
        if value.kind() != "quoted_attribute_value" {
            return Err("dynamic brand attributes are unqualified".into());
        }
        if self
            .0
            .children(*value)
            .iter()
            .any(|child| child.kind() == "expression")
        {
            return Err("interpolated brand attributes are unqualified".into());
        }
        let text = self.0.text(*value);
        let literal = text
            .get(1..text.len().saturating_sub(1))
            .ok_or("invalid brand attribute")?;
        if literal.contains(['&', '{', '}', '\\']) {
            return Err("encoded brand attributes are unqualified".into());
        }
        *target = Some(literal.into());
        Ok(())
    }
}

impl Mark {
    pub(super) fn verify(
        &self,
        catalog: &super::receipt::Catalog,
        file: &std::path::Path,
    ) -> Result<(), String> {
        catalog.product(&self.product)?;
        if let Some(surface) = &self.surface {
            let supported = catalog
                .assets
                .values()
                .any(|asset| asset.product == self.product && asset.surface == *surface);
            if !supported {
                return Err(format!(
                    "{} selects an unsupported formal surface",
                    file.display()
                ));
            }
        }
        Ok(())
    }
}
