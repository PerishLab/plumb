use crate::shape::web::Syntax;
use std::collections::BTreeMap;
use tree_sitter::{Node, Parser};

pub(super) struct Document {
    pub(super) script: String,
}

pub(super) fn read(text: &str) -> Result<Document, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_svelte_next::LANGUAGE.into())
        .map_err(|error| error.to_string())?;
    let tree = parser
        .parse(text, None)
        .ok_or("Views HTML entry was not parsed")?;
    if tree.root_node().has_error() {
        return Err("Views HTML entry syntax is unqualified".into());
    }
    let mut found = Html {
        syntax: Syntax(text),
        scripts: Vec::new(),
        roots: 0,
    };
    found.walk(tree.root_node())?;
    if found.roots != 1 || found.scripts.len() != 1 {
        return Err(
            "Views HTML needs exactly one empty root div and one actual module script".into(),
        );
    }
    Ok(Document {
        script: found.scripts.remove(0),
    })
}

struct Html<'a> {
    syntax: Syntax<'a>,
    scripts: Vec<String>,
    roots: usize,
}

impl Html<'_> {
    fn walk(&mut self, node: Node<'_>) -> Result<(), String> {
        if node.kind() == "script_element" {
            return self.script(node);
        }
        if node.kind() == "style_element" {
            return Err("inline HTML transforms/styles are outside the Views carrier".into());
        }
        if node.kind() == "element" {
            self.element(node)?;
        }
        if ["start_tag", "self_closing_tag"].contains(&node.kind()) {
            self.tag(node)?;
        }
        for child in self.syntax.children(node) {
            self.walk(child)?;
        }
        Ok(())
    }

    fn element(&mut self, node: Node<'_>) -> Result<(), String> {
        let children = self.syntax.children(node);
        let Some(start) = children.iter().find(|node| node.kind() == "start_tag") else {
            return Ok(());
        };
        if self.name(*start) != "div" {
            return Ok(());
        }
        let attributes = self.attributes(*start)?;
        if attributes.len() != 1 || attributes.get("id").map(String::as_str) != Some("root") {
            return Err("Views carrier supports only its unmodified root div".into());
        }
        if children.iter().any(|node| {
            !["start_tag", "end_tag"].contains(&node.kind())
                && !self.syntax.text(*node).trim().is_empty()
        }) {
            return Err("Views mount root is not empty".into());
        }
        let parent = node.parent().ok_or("Views root has no body")?;
        let start = self
            .syntax
            .children(parent)
            .into_iter()
            .find(|node| node.kind() == "start_tag")
            .ok_or("Views root is outside the body")?;
        if self.name(start) != "body" {
            return Err("Views root must be an ordinary direct body child".into());
        }
        self.roots += 1;
        Ok(())
    }

    fn tag(&self, node: Node<'_>) -> Result<(), String> {
        let name = self.name(node);
        if !["html", "head", "body", "meta", "link", "title", "div"].contains(&name) {
            return Err("extra HTML elements are outside the qualified Views carrier".into());
        }
        let attributes = self.attributes(node)?;
        if attributes.keys().any(|name| name.starts_with("on"))
            || attributes.contains_key("http-equiv")
        {
            return Err("HTML event or navigation transforms are outside the Views carrier".into());
        }
        Ok(())
    }

    fn script(&mut self, node: Node<'_>) -> Result<(), String> {
        let children = self.syntax.children(node);
        let start = children
            .iter()
            .find(|node| node.kind() == "start_tag")
            .ok_or("script has no opening tag")?;
        let attributes = self.attributes(*start)?;
        if attributes.get("type").map(String::as_str) != Some("module")
            || attributes
                .keys()
                .any(|name| !["type", "src", "crossorigin"].contains(&name.as_str()))
        {
            return Err(
                "Views requires an ordinary module script without inline transforms".into(),
            );
        }
        if children
            .iter()
            .any(|node| node.kind() == "raw_text" && !self.syntax.text(*node).trim().is_empty())
        {
            return Err("inline HTML programs are outside the Views carrier".into());
        }
        let source = attributes
            .get("src")
            .ok_or("Views module script has no source")?;
        let path = source
            .strip_prefix('/')
            .ok_or("Views module script must be local and absolute")?;
        super::relative(path)?;
        self.scripts.push(source.clone());
        Ok(())
    }

    fn name(&self, node: Node<'_>) -> &str {
        self.syntax
            .children(node)
            .into_iter()
            .find(|node| node.kind() == "tag_name")
            .map(|node| self.syntax.text(node))
            .unwrap_or_default()
    }

    fn attributes(&self, node: Node<'_>) -> Result<BTreeMap<String, String>, String> {
        let mut found = BTreeMap::new();
        for attribute in self
            .syntax
            .children(node)
            .into_iter()
            .filter(|node| node.kind() != "tag_name")
        {
            let parts = self.syntax.children(attribute);
            let name = parts
                .first()
                .filter(|node| node.kind() == "attribute_name")
                .ok_or("dynamic HTML attribute is unqualified")?;
            let value = parts
                .get(1)
                .map(|node| self.syntax.text(*node))
                .unwrap_or_default();
            let value = if value.starts_with(['\'', '"']) {
                &value[1..value.len().saturating_sub(1)]
            } else {
                value
            };
            if value.contains(['&', '{', '}', '\\']) {
                return Err("encoded/dynamic HTML entry attribute is unqualified".into());
            }
            if found
                .insert(self.syntax.text(*name).to_lowercase(), value.into())
                .is_some()
            {
                return Err("duplicate HTML entry attribute".into());
            }
        }
        Ok(found)
    }
}
