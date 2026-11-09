use super::syntax::Syntax;
use std::collections::BTreeMap;
use tree_sitter::{Node, Parser};

#[derive(Debug, PartialEq)]
pub(crate) enum Activation {
    Active(Setup),
    Missing,
    Unknown(String),
}

impl Activation {
    pub(crate) fn read(text: &str) -> Self {
        match Vite(Syntax(text)).read() {
            Ok(value) => value,
            Err(reason) => Self::Unknown(reason),
        }
    }
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize, Clone)]
pub(crate) struct Setup {
    pub(crate) options: serde_json::Value,
    pub(crate) base: String,
    pub(crate) output: String,
    pub(crate) problem: Option<String>,
}

struct Vite<'a>(Syntax<'a>);

impl Vite<'_> {
    fn read(&self) -> Result<Activation, String> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .map_err(|error| error.to_string())?;
        let tree = parser
            .parse(self.0.0, None)
            .ok_or("Vite configuration could not be parsed")?;
        let root = tree.root_node();
        if root.has_error() {
            return Err("Vite configuration contains unsupported or invalid syntax".into());
        }
        let mut bindings = BTreeMap::new();
        let mut exported = None;
        let mut uncertain = false;
        for node in self.0.children(root) {
            match node.kind() {
                "import_statement" => {
                    let source = node
                        .child_by_field_name("source")
                        .and_then(|node| self.0.string(node))
                        .unwrap_or_default();
                    uncertain |= ![
                        "vite",
                        "@perishlab/design/vite",
                        "@sveltejs/vite-plugin-svelte",
                    ]
                    .contains(&source.as_str());
                    self.0.import(node, &mut bindings)?;
                }
                "export_statement" if exported.is_none() => {
                    exported = node.child_by_field_name("value")
                }
                _ => {
                    return Err(
                        "only imports and one direct default configuration are supported".into(),
                    );
                }
            }
        }
        let Some(mut config) = exported else {
            return Ok(Activation::Missing);
        };
        if config.kind() == "call_expression" {
            let name = config
                .child_by_field_name("function")
                .ok_or("missing config function")?;
            if bindings.get(self.0.text(name)).map(String::as_str) != Some("vite#defineConfig") {
                return Err("default config call is not the imported Vite defineConfig".into());
            }
            config = self.argument(config)?.ok_or("defineConfig has no object")?;
        }
        let Some(plugins) = self.0.property(config, "plugins")? else {
            return Ok(Activation::Missing);
        };
        if plugins.kind() != "array" {
            return Err("plugins must be a literal array".into());
        }
        let mut active = None;
        let mut foreign = uncertain;
        for plugin in self.0.children(plugins) {
            if plugin.kind() != "call_expression" {
                return Err("dynamic plugin entries and spreads are unsupported".into());
            }
            let function = plugin
                .child_by_field_name("function")
                .ok_or("missing plugin function")?;
            if bindings.get(self.0.text(function)).map(String::as_str)
                == Some("@perishlab/design/vite#design")
            {
                if active.is_some() {
                    return Err("multiple design plugin activations are unsupported".into());
                }
                active = Some(self.options(plugin)?);
            } else if bindings.get(self.0.text(function)).map(String::as_str)
                != Some("@sveltejs/vite-plugin-svelte#svelte")
            {
                foreign = true;
            }
        }
        let Some(options) = active else {
            return Ok(Activation::Missing);
        };
        let base = self.string(config, "base")?.unwrap_or("/".into());
        let output = match self.0.property(config, "build")? {
            Some(build) => self.string(build, "outDir")?.unwrap_or("dist".into()),
            None => "dist".into(),
        };
        Ok(Activation::Active(Setup {
            options,
            base,
            output,
            problem: self.scope(config, foreign).err(),
        }))
    }

    fn scope(&self, config: Node<'_>, foreign: bool) -> Result<(), String> {
        if foreign {
            return Err(
                "additional plugin implementations are outside qualified brand scope".into(),
            );
        }
        for name in self.0.names(config)? {
            if !["plugins", "base", "build"].contains(&name.as_str()) {
                return Err(format!(
                    "Vite {name} is outside qualified brand scope; aliases/transforms cannot establish origin"
                ));
            }
        }
        if let Some(build) = self.0.property(config, "build")? {
            for name in self.0.names(build)? {
                if name != "outDir" {
                    return Err(format!(
                        "Vite build.{name} is outside qualified brand scope"
                    ));
                }
            }
        }
        Ok(())
    }

    fn string(&self, object: Node<'_>, key: &str) -> Result<Option<String>, String> {
        self.0
            .property(object, key)?
            .map(|node| {
                self.0
                    .string(node)
                    .ok_or_else(|| format!("dynamic {key} is unsupported"))
            })
            .transpose()
    }

    fn options(&self, plugin: Node<'_>) -> Result<serde_json::Value, String> {
        match self.argument(plugin)? {
            Some(node) => self.0.literal(node),
            None => Ok(serde_json::json!({})),
        }
    }

    fn argument<'a>(&self, call: Node<'a>) -> Result<Option<Node<'a>>, String> {
        let arguments = call
            .child_by_field_name("arguments")
            .ok_or("missing call arguments")?;
        let nodes = self.0.children(arguments);
        if nodes.len() > 1 {
            return Err("multiple config arguments are unsupported".into());
        }
        Ok(nodes.first().copied())
    }
}

#[cfg(test)]
mod tests;
