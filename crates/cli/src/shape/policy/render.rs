use super::Expected;
use std::collections::BTreeSet;
use std::path::Path;

pub fn render(root: &Path, text: &str) -> Result<String, String> {
    let mut doc = text
        .parse::<toml::Table>()
        .map_err(|error| error.to_string())?;
    let want = Expected::read(root);
    doc.insert(
        "scan".to_string(),
        toml::Value::Table(toml::Table::from_iter([
            ("include".to_string(), values(&want.include)),
            ("exclude".to_string(), values(&want.exclude)),
        ])),
    );
    doc.insert(
        "module".to_string(),
        toml::Value::Table(toml::Table::from_iter([(
            "roots".to_string(),
            values(&want.roots),
        )])),
    );
    doc.insert(
        "limit".to_string(),
        toml::Value::Table(
            crate::catalog::set::LIMITS
                .iter()
                .map(|(name, value)| (name.clone(), toml::Value::Integer(*value)))
                .collect(),
        ),
    );
    doc.insert(
        "comment".to_string(),
        toml::Value::Table(toml::Table::from_iter([(
            "allow".to_string(),
            toml::Value::Boolean(false),
        )])),
    );
    doc.insert(
        "word".to_string(),
        toml::Value::Table(toml::Table::from_iter([(
            "single".to_string(),
            toml::Value::Boolean(true),
        )])),
    );
    merge(&mut doc, ("grant", "test"), want.tests.clone(), false);
    merge(&mut doc, ("grant", "environment"), want.tests, true);
    merge(&mut doc, ("ban", "style"), want.bans, true);
    toml::to_string_pretty(&doc).map_err(|error| error.to_string())
}

fn merge(doc: &mut toml::Table, syntax: (&str, &str), mut paths: BTreeSet<String>, preserve: bool) {
    let (table, name) = syntax;
    let mut kept = Vec::new();
    let entries = doc
        .remove(table)
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    for entry in entries {
        let held = entry.as_table();
        let matches = held
            .and_then(|value| value.get("syntax"))
            .and_then(toml::Value::as_str)
            == Some(name);
        if matches {
            if preserve
                && let Some(values) = held
                    .and_then(|value| value.get("paths"))
                    .and_then(toml::Value::as_array)
            {
                paths.extend(
                    values
                        .iter()
                        .filter_map(toml::Value::as_str)
                        .map(str::to_string),
                );
            }
        } else {
            kept.push(entry);
        }
    }
    if !paths.is_empty() {
        kept.push(toml::Value::Table(toml::Table::from_iter([
            ("syntax".to_string(), toml::Value::String(name.to_string())),
            ("paths".to_string(), values(&paths)),
        ])));
    }
    if !kept.is_empty() {
        doc.insert(table.to_string(), toml::Value::Array(kept));
    }
}

fn values(set: &BTreeSet<String>) -> toml::Value {
    toml::Value::Array(set.iter().cloned().map(toml::Value::String).collect())
}
