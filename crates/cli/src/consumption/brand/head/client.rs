use super::{Asset, Head, Selection};
use crate::shape::web::Syntax;
use std::collections::BTreeMap;
use std::path::Path;
use tree_sitter::{Node, Parser};

pub(super) fn verify(
    root: &Path,
    selection: Option<&Selection>,
    outputs: &BTreeMap<String, Vec<Asset>>,
) -> Result<(), String> {
    let files = super::documents(root)?;
    if files != [root.join("index.html")] {
        return Err("Views carrier requires exactly one built root HTML document".into());
    }
    let text = String::from_utf8(super::super::material::regular(root, "index.html")?)
        .map_err(|error| error.to_string())?;
    if let Some(selection) = selection {
        super::read(&text, selection, outputs)?;
        return Ok(());
    }
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_svelte_next::LANGUAGE.into())
        .map_err(|error| error.to_string())?;
    let tree = parser
        .parse(&text, None)
        .ok_or("headless HTML was not parsed")?;
    if tree.root_node().has_error() {
        return Err("headless HTML syntax is unqualified".into());
    }
    Empty(Head {
        syntax: Syntax(&text),
        origin: "",
        product: "",
        outputs,
    })
    .walk(tree.root_node(), false)
}

struct Empty<'a>(Head<'a>);

impl Empty<'_> {
    fn walk(&self, node: Node<'_>, inside: bool) -> Result<(), String> {
        let children = self.0.syntax.children(node);
        let start = children.iter().find(|node| node.kind() == "start_tag");
        let inside = inside || start.is_some_and(|node| self.name(*node) == "head");
        if ["start_tag", "self_closing_tag"].contains(&node.kind()) {
            if inside {
                self.tag(node)?;
            } else {
                self.0.mark(node)?;
            }
        }
        if ["script_element", "style_element", "comment"].contains(&node.kind()) {
            return Ok(());
        }
        for child in children {
            self.walk(child, inside)?;
        }
        Ok(())
    }

    fn tag(&self, node: Node<'_>) -> Result<(), String> {
        let attributes = self.0.attributes(node)?;
        match self.name(node) {
            "link" => {
                super::media::link(&attributes)?;
                let rel = attributes
                    .get("rel")
                    .map(|value| value.to_ascii_lowercase())
                    .unwrap_or_default();
                if ["icon", "shortcut icon", "apple-touch-icon"].contains(&rel.as_str())
                    && (rel == "apple-touch-icon"
                        || attributes.get("href").map(String::as_str) != Some("data:,"))
                {
                    return Err(
                        "headless consumption contains an undeclared graphic head icon".into(),
                    );
                }
            }
            "meta" => {
                super::media::meta(&attributes)?;
                let key = attributes
                    .get("property")
                    .or_else(|| attributes.get("name"))
                    .map(|value| value.to_ascii_lowercase())
                    .unwrap_or_default();
                if key.starts_with("og:image")
                    || key.starts_with("twitter:image")
                    || key == "twitter:card"
                {
                    return Err(
                        "headless consumption contains undeclared sharing image metadata".into(),
                    );
                }
            }
            _ => (),
        }
        Ok(())
    }

    fn name(&self, node: Node<'_>) -> &str {
        self.0
            .syntax
            .children(node)
            .into_iter()
            .find(|node| node.kind() == "tag_name")
            .map(|node| self.0.syntax.text(node))
            .unwrap_or_default()
    }
}
