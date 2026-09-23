use super::walk::Held;
use std::collections::{BTreeMap, BTreeSet};

const SELECTS: [&str; 10] = [
    "layout", "seat", "file", "shape", "web", "when", "use", "svelte", "tsx", "path",
];
const TELLS: [&str; 3] = ["name", "note", "kind"];

pub fn judge(held: &Held) -> Result<(), String> {
    let mechanized = mechanized(held)?;
    let members = members(held);
    let mut taken = BTreeSet::new();
    for (name, table) in &held.files {
        if name.starts_with("atoms/") {
            bound(name, table, &mechanized)?;
        }
        if name.starts_with("suites/") {
            selects(name, table)?;
            taken.extend(referenced(table));
        }
    }
    for held in &taken {
        if !members.contains_key(held) {
            return Err(format!("{held} names no atom"));
        }
    }
    idle(&members, &taken)
}

fn idle(members: &BTreeMap<String, bool>, taken: &BTreeSet<String>) -> Result<(), String> {
    let missed = members
        .iter()
        .filter(|(held, retired)| !**retired && !taken.contains(*held))
        .map(|(held, _)| held.clone())
        .collect::<Vec<_>>();
    if missed.is_empty() {
        return Ok(());
    }
    Err(format!(
        "no suite selects {}; select it or retire it",
        missed.join(", ")
    ))
}

fn mechanized(held: &Held) -> Result<BTreeSet<String>, String> {
    let law = held
        .files
        .iter()
        .find(|(name, _)| name == "catalog.toml")
        .ok_or_else(|| "rules/catalog.toml is the catalogue and is owed".to_string())?;
    Ok(law
        .1
        .get("rule")
        .and_then(toml::Value::as_array)
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|rule| rule.get("standing").and_then(toml::Value::as_str) == Some("mechanized"))
        .filter_map(|rule| rule.get("id").and_then(toml::Value::as_str))
        .map(str::to_string)
        .collect())
}

fn bound(name: &str, table: &toml::Table, mechanized: &BTreeSet<String>) -> Result<(), String> {
    let named = Read(&toml::Value::Table(table.clone())).rules();
    if named.is_empty() {
        return Err(format!("rules/{name} names no rule it judges"));
    }
    for held in named {
        if !mechanized.contains(&held) {
            return Err(format!(
                "rules/{name} judges {held}, which the catalogue does not call mechanized"
            ));
        }
    }
    Ok(())
}

struct Read<'a>(&'a toml::Value);

impl Read<'_> {
    fn rules(&self) -> BTreeSet<String> {
        let mut held = BTreeSet::new();
        if let Some(table) = self.0.as_table() {
            if let Some(rule) = table.get("rule").and_then(toml::Value::as_str) {
                held.insert(rule.to_string());
            }
            held.extend(table.values().flat_map(|value| Read(value).rules()));
        }
        if let Some(list) = self.0.as_array() {
            held.extend(list.iter().flat_map(|value| Read(value).rules()));
        }
        held
    }

    fn keys(&self) -> BTreeSet<String> {
        let mut held = BTreeSet::new();
        if let Some(table) = self.0.as_table() {
            held.extend(table.keys().cloned());
            held.extend(table.values().flat_map(|value| Read(value).keys()));
        }
        if let Some(list) = self.0.as_array() {
            held.extend(list.iter().flat_map(|value| Read(value).keys()));
        }
        held
    }

    fn strings(&self) -> BTreeSet<String> {
        match self.0 {
            toml::Value::String(held) => BTreeSet::from([held.clone()]),
            toml::Value::Table(table) => table
                .values()
                .flat_map(|value| Read(value).strings())
                .collect(),
            toml::Value::Array(list) => list
                .iter()
                .flat_map(|value| Read(value).strings())
                .collect(),
            _ => BTreeSet::new(),
        }
    }

    fn named(&self) -> Vec<(String, bool)> {
        let mut held = Vec::new();
        if let Some(table) = self.0.as_table() {
            if let Some(name) = table.get("name").and_then(toml::Value::as_str) {
                let retired = table.get("kind").and_then(toml::Value::as_str) == Some("retired");
                held.push((name.to_string(), retired));
            }
            held.extend(table.values().flat_map(|value| Read(value).named()));
        }
        if let Some(list) = self.0.as_array() {
            held.extend(list.iter().flat_map(|value| Read(value).named()));
        }
        held
    }
}

fn selects(name: &str, table: &toml::Table) -> Result<(), String> {
    for key in Read(&toml::Value::Table(table.clone())).keys() {
        let known = SELECTS.contains(&key.as_str()) || TELLS.contains(&key.as_str());
        if !known && key != "rule" {
            return Err(format!(
                "rules/{name} carries {key}; a suite selects atoms and asserts nothing itself"
            ));
        }
    }
    Ok(())
}

fn referenced(table: &toml::Table) -> BTreeSet<String> {
    Read(&toml::Value::Table(table.clone()))
        .strings()
        .into_iter()
        .filter(|held| held.starts_with("rule://"))
        .collect()
}

fn members(held: &Held) -> BTreeMap<String, bool> {
    let mut found = BTreeMap::new();
    for (name, table) in &held.files {
        let Some(set) = name
            .strip_prefix("atoms/")
            .and_then(|name| name.strip_suffix(".toml"))
        else {
            continue;
        };
        if table.get("addressed").and_then(toml::Value::as_bool) != Some(true) {
            continue;
        }
        for (slug, retired) in Read(&toml::Value::Table(table.clone())).named() {
            found.insert(format!("rule://{set}/{slug}"), retired);
        }
    }
    found
}
