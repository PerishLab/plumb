use super::receipt::{Asset, Selection};
use crate::shape::web::Syntax;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser};

pub(super) fn client(
    root: &Path,
    selection: Option<&Selection>,
    outputs: &BTreeMap<String, Vec<Asset>>,
) -> Result<(), String> {
    client::verify(root, selection, outputs)
}

pub(super) fn verify(
    root: &Path,
    selection: &Selection,
    outputs: &BTreeMap<String, Vec<Asset>>,
) -> Result<(), String> {
    let files = documents(root)?;
    if files.is_empty() {
        return Err("brand output has no actual HTML document".into());
    }
    let mut rendered = 0;
    for file in files {
        let path = file
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_str()
            .ok_or("non-text HTML path")?;
        let bytes = super::material::regular(root, path)?;
        let text = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
        rendered += read(text, selection, outputs)
            .map_err(|error| format!("{}: {error}", file.display()))?;
    }
    if rendered == 0 {
        return Err("HTML output has no qualified rendered brand image; CSR-only or unreachable source remains unqualified".into());
    }
    Ok(())
}

pub(super) fn read(
    text: &str,
    selection: &Selection,
    outputs: &BTreeMap<String, Vec<Asset>>,
) -> Result<usize, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_svelte_next::LANGUAGE.into())
        .map_err(|error| error.to_string())?;
    let tree = parser
        .parse(text, None)
        .ok_or("HTML output was not parsed")?;
    if tree.root_node().has_error() {
        return Err("HTML brand output syntax is unqualified".into());
    }
    let reader = Head {
        syntax: Syntax(text),
        origin: &selection.origin,
        product: &selection.product,
        outputs,
    };
    let mut found = Observed {
        head: BTreeMap::new(),
        rendered: 0,
    };
    reader.walk(tree.root_node(), false, &mut found)?;
    for required in ["icon", "apple-touch-icon", "og:image", "twitter:image"] {
        if !found.head.contains_key(required) {
            return Err(format!("HTML head omits formal {required}"));
        }
    }
    media::dimensions(
        &found.head,
        reader.product(found.head.get("og:image").ok_or("missing sharing image")?)?,
    )?;
    Ok(found.rendered)
}

struct Observed {
    head: BTreeMap<String, String>,
    rendered: usize,
}

struct Head<'a> {
    syntax: Syntax<'a>,
    origin: &'a str,
    product: &'a str,
    outputs: &'a BTreeMap<String, Vec<Asset>>,
}

impl Head<'_> {
    fn walk(&self, node: Node<'_>, inside: bool, found: &mut Observed) -> Result<(), String> {
        let children = self.syntax.children(node);
        let start = children.iter().find(|child| child.kind() == "start_tag");
        let name = start.and_then(|start| {
            self.syntax
                .children(*start)
                .into_iter()
                .find(|child| child.kind() == "tag_name")
        });
        let inside = inside || name.is_some_and(|node| self.syntax.text(node) == "head");
        if inside && ["start_tag", "self_closing_tag"].contains(&node.kind()) {
            self.tag(node, &mut found.head)?;
        }
        if !inside && ["start_tag", "self_closing_tag"].contains(&node.kind()) {
            found.rendered += self.mark(node)?;
        }
        if ["script_element", "style_element", "comment"].contains(&node.kind()) {
            return Ok(());
        }
        for child in children {
            self.walk(child, inside, found)?;
        }
        Ok(())
    }

    fn tag(&self, node: Node<'_>, found: &mut BTreeMap<String, String>) -> Result<(), String> {
        let children = self.syntax.children(node);
        let name = children
            .iter()
            .find(|child| child.kind() == "tag_name")
            .map(|node| self.syntax.text(*node))
            .unwrap_or_default();
        if !["link", "meta"].contains(&name) {
            return Ok(());
        }
        let attributes = self.attributes(node)?;
        let value = |key: &str| attributes.get(key).map(String::as_str).unwrap_or_default();
        if name == "link" {
            media::link(&attributes)?;
            return self.icon(&attributes, found);
        }
        media::meta(&attributes)?;
        let key = attributes
            .get("property")
            .or_else(|| attributes.get("name"))
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();
        if ["og:image:width", "og:image:height"].contains(&key.as_str())
            && found.insert(key.clone(), value("content").into()).is_some()
        {
            return Err("duplicate sharing image dimension".into());
        }
        if ["og:image", "twitter:image"].contains(&key.as_str()) {
            let asset = self.product(value("content"))?;
            if asset.medium != "png" || asset.width != asset.height {
                return Err(
                    "sharing image is outside the current formal square PNG contract".into(),
                );
            }
            if found.insert(key.clone(), value("content").into()).is_some() {
                return Err("duplicate brand head metadata".into());
            }
        }
        if key == "twitter:card" && value("content") != "summary" {
            return Err(
                "sharing metadata claims a format outside the current square-image contract".into(),
            );
        }
        Ok(())
    }

    fn icon(
        &self,
        attributes: &BTreeMap<String, String>,
        found: &mut BTreeMap<String, String>,
    ) -> Result<(), String> {
        let rel = attributes
            .get("rel")
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();
        if ["mask-icon", "apple-touch-icon-precomposed"].contains(&rel.as_str()) {
            return Err("head icon format is unqualified".into());
        }
        if !["icon", "shortcut icon", "apple-touch-icon"].contains(&rel.as_str()) {
            return Ok(());
        }
        let key = if rel == "shortcut icon" {
            "icon"
        } else {
            rel.as_str()
        };
        let reference = attributes
            .get("href")
            .map(String::as_str)
            .unwrap_or_default();
        let asset = self.product(reference)?;
        if key == "apple-touch-icon" && asset.medium != "png" {
            return Err("touch icon is outside the formal PNG contract".into());
        }
        found.insert(key.into(), reference.into());
        Ok(())
    }

    fn image(&self, reference: &str) -> Result<&[Asset], String> {
        let path = reference.strip_prefix(self.origin).unwrap_or(reference);
        self.outputs.get(path).map(Vec::as_slice).ok_or_else(|| {
            format!("HTML brand reference {reference:?} is outside its formal product outputs")
        })
    }

    fn product(&self, reference: &str) -> Result<&Asset, String> {
        self.image(reference)?
            .iter()
            .find(|asset| asset.product == self.product)
            .ok_or("HTML head selects a different formal product".into())
    }

    fn mark(&self, node: Node<'_>) -> Result<usize, String> {
        let attributes = self.attributes(node)?;
        let Some(id) = attributes.get("data-crest") else {
            return Ok(0);
        };
        let name = self
            .syntax
            .children(node)
            .into_iter()
            .find(|child| child.kind() == "tag_name")
            .map(|node| self.syntax.text(node))
            .unwrap_or_default();
        if name != "img" {
            return Err("declared rendered brand is outside the official image renderer".into());
        }
        let reference = attributes
            .get("src")
            .ok_or("rendered brand image has no source")?;
        let asset = self
            .image(reference)?
            .iter()
            .find(|asset| asset.id == *id)
            .ok_or("rendered brand ID has no matching formal output")?;
        if attributes.get("data-kind") != Some(&asset.kind) {
            return Err("rendered brand identity differs from its formal image selection".into());
        }
        Ok(1)
    }

    fn attributes(&self, node: Node<'_>) -> Result<BTreeMap<String, String>, String> {
        let mut attributes = BTreeMap::new();
        for attribute in self
            .syntax
            .children(node)
            .into_iter()
            .filter(|node| node.kind() != "tag_name")
        {
            let parts = self.syntax.children(attribute);
            let Some(name) = parts.first() else {
                return Err("HTML attribute is unqualified".into());
            };
            let name = self.syntax.text(*name).to_ascii_lowercase();
            let text = parts
                .get(1)
                .map(|node| self.syntax.text(*node))
                .unwrap_or_default();
            let text = if text.starts_with(['\"', '\'']) {
                &text[1..text.len().saturating_sub(1)]
            } else {
                text
            };
            if attributes.insert(name, text.into()).is_some() {
                return Err("duplicate HTML head attribute".into());
            }
        }
        Ok(attributes)
    }
}

fn documents(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_symlink() {
            return Err("symlinked brand output is unqualified".into());
        }
        if kind.is_dir() {
            found.extend(documents(&entry.path())?);
        }
        if kind.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "html")
        {
            found.push(entry.path());
        }
        if found.len() > 4096 {
            return Err("HTML brand inventory exceeded its bound".into());
        }
    }
    found.sort();
    Ok(found)
}
mod client;
mod media;
