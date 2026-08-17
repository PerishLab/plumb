use std::collections::BTreeMap;
use std::path::Path;
use toml_edit::{DocumentMut, Item, Value};

pub fn dependencies(
    document: &mut DocumentMut,
    pins: &BTreeMap<String, String>,
) -> Result<(), String> {
    if let Some(table) = document
        .get_mut("workspace")
        .and_then(Item::as_table_mut)
        .and_then(|workspace| workspace.get_mut("dependencies"))
        .and_then(Item::as_table_mut)
    {
        requirements(table, pins);
    }
    for section in ["dependencies", "build-dependencies", "dev-dependencies"] {
        let Some(table) = document.get_mut(section).and_then(Item::as_table_mut) else {
            continue;
        };
        requirements(table, pins);
    }
    Ok(())
}

pub fn requirements(table: &mut toml_edit::Table, pins: &BTreeMap<String, String>) {
    for (name, item) in table.iter_mut() {
        dependency(&name, item, pins);
    }
}

pub fn dependency(name: &str, item: &mut Item, pins: &BTreeMap<String, String>) {
    if let Some(detail) = item.as_inline_table_mut() {
        let identity = detail
            .get("package")
            .and_then(Value::as_str)
            .unwrap_or(name);
        if let Some(pin) = pins.get(identity).filter(|_| detail.contains_key("path")) {
            detail.insert("version", Value::from(format!("={pin}")));
        }
    } else if let Some(detail) = item.as_table_mut() {
        let identity = detail.get("package").and_then(Item::as_str).unwrap_or(name);
        if let Some(pin) = pins.get(identity).filter(|_| detail.contains_key("path")) {
            detail["version"] = toml_edit::value(format!("={pin}"));
        }
    }
}

pub fn read(path: &Path) -> Result<DocumentMut, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?
        .parse()
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))
}

pub fn write(path: &Path, document: &DocumentMut) -> Result<(), String> {
    std::fs::write(path, document.to_string())
        .map_err(|error| format!("cannot stamp {}: {error}", path.display()))
}
