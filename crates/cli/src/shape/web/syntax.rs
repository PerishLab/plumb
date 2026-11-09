use std::collections::BTreeMap;
use tree_sitter::Node;

pub(crate) struct Syntax<'a>(pub(crate) &'a str);

impl<'a> Syntax<'a> {
    pub(crate) fn text(&self, node: Node<'_>) -> &'a str {
        &self.0[node.byte_range()]
    }

    pub(crate) fn children<'b>(&self, node: Node<'b>) -> Vec<Node<'b>> {
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .filter(|child| child.kind() != "comment")
            .collect()
    }

    pub(crate) fn string(&self, node: Node<'_>) -> Option<String> {
        if node.kind() != "string" {
            return None;
        }
        let text = self.text(node);
        let body = text.get(1..text.len().checked_sub(1)?)?;
        if body.contains(['\\', '\n', '\r']) {
            return None;
        }
        Some(body.to_string())
    }

    pub(crate) fn property<'b>(
        &self,
        node: Node<'b>,
        name: &str,
    ) -> Result<Option<Node<'b>>, String> {
        if node.kind() != "object" {
            return Err("configuration is not a static object".into());
        }
        let mut found = None;
        for child in self.children(node) {
            if child.kind() != "pair" {
                return Err("spread, shorthand or method configuration is unsupported".into());
            }
            let key = child
                .child_by_field_name("key")
                .ok_or("missing object key")?;
            let text = match key.kind() {
                "property_identifier" => self.text(key).to_string(),
                "string" => self
                    .string(key)
                    .ok_or("escaped object key is unsupported")?,
                _ => return Err("computed object key is unsupported".into()),
            };
            if text == name {
                if found.is_some() {
                    return Err(format!("duplicate configuration key {name}"));
                }
                found = child.child_by_field_name("value");
            }
        }
        Ok(found)
    }

    pub(crate) fn names(&self, object: Node<'_>) -> Result<Vec<String>, String> {
        if object.kind() != "object" {
            return Err("configuration is not a static object".into());
        }
        let mut names = Vec::new();
        for pair in self.children(object) {
            let key = pair
                .child_by_field_name("key")
                .ok_or("non-pair configuration is unsupported")?;
            let name = match key.kind() {
                "property_identifier" => self.text(key).to_string(),
                "string" => self
                    .string(key)
                    .ok_or("encoded configuration key is unsupported")?,
                _ => return Err("computed configuration key is unsupported".into()),
            };
            if names.contains(&name) {
                return Err("duplicate configuration key".into());
            }
            names.push(name);
        }
        Ok(names)
    }

    pub(crate) fn literal(&self, node: Node<'_>) -> Result<serde_json::Value, String> {
        match node.kind() {
            "string" => self
                .string(node)
                .map(Into::into)
                .ok_or("escaped string is unsupported".into()),
            "true" => Ok(true.into()),
            "false" => Ok(false.into()),
            "null" => Ok(serde_json::Value::Null),
            "number" => serde_json::from_str(self.text(node))
                .map_err(|_| "non-JSON number is unsupported".into()),
            "object" => self.object(node),
            _ => Err(format!("dynamic {} value is unsupported", node.kind())),
        }
    }

    fn object(&self, node: Node<'_>) -> Result<serde_json::Value, String> {
        let mut result = serde_json::Map::new();
        for child in self.children(node) {
            if child.kind() != "pair" {
                return Err("spread, shorthand or method value is unsupported".into());
            }
            let key = child
                .child_by_field_name("key")
                .ok_or("missing literal key")?;
            let name = if key.kind() == "property_identifier" {
                self.text(key).to_string()
            } else {
                self.string(key)
                    .ok_or("computed literal key is unsupported")?
            };
            let value = child
                .child_by_field_name("value")
                .ok_or("missing literal value")?;
            if result.insert(name, self.literal(value)?).is_some() {
                return Err("duplicate literal key".into());
            }
        }
        Ok(result.into())
    }
    pub(crate) fn import(
        &self,
        node: Node<'_>,
        bindings: &mut BTreeMap<String, String>,
    ) -> Result<(), String> {
        let source = node
            .child_by_field_name("source")
            .and_then(|node| self.string(node))
            .ok_or("escaped or dynamic import source is unsupported")?;
        if self.typed(node) {
            return Ok(());
        }
        let clause = self
            .children(node)
            .into_iter()
            .find(|node| node.kind() == "import_clause");
        let Some(clause) = clause else { return Ok(()) };
        for part in self.children(clause) {
            if part.kind() != "named_imports" {
                continue;
            }
            for item in self.children(part) {
                if self.typed(item) {
                    continue;
                }
                let name = item
                    .child_by_field_name("name")
                    .ok_or("missing import name")?;
                let alias = item.child_by_field_name("alias").unwrap_or(name);
                let value = format!("{source}#{}", self.text(name));
                if bindings
                    .insert(self.text(alias).to_string(), value)
                    .is_some()
                {
                    return Err("duplicate imported local binding".into());
                }
            }
        }
        Ok(())
    }

    fn typed(&self, node: Node<'_>) -> bool {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .any(|child| child.kind() == "type")
    }
}
