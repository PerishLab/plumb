use crate::catalog::set;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub struct Key {
    pub segments: Vec<String>,
    pub roots: Vec<String>,
    pub paths: Vec<String>,
}

impl Key {
    pub fn lane(&self) -> String {
        self.segments.first().cloned().unwrap_or_default()
    }

    pub fn output(&self) -> String {
        self.segments
            .iter()
            .skip(1)
            .cloned()
            .collect::<Vec<_>>()
            .join("_")
            .chars()
            .map(|held| {
                if held.is_ascii_alphanumeric() {
                    held
                } else {
                    '_'
                }
            })
            .collect()
    }

    pub fn name(&self) -> String {
        match self.segments.split_first() {
            Some((lane, [])) => lane.clone(),
            Some((lane, rest)) => format!("{lane}/{}", rest.join(".")),
            None => String::new(),
        }
    }
}

#[derive(Default)]
pub struct Held {
    pub keys: Vec<Key>,
    pub refusal: Option<String>,
}

impl Held {
    pub fn contribution(&self, root: &str) -> Vec<String> {
        if let Some(name) = root.strip_prefix("key://") {
            return self
                .keys
                .iter()
                .find(|key| key.name() == name)
                .map(|key| key.paths.clone())
                .unwrap_or_default();
        }
        if let Some(name) = root.strip_prefix("suite://") {
            return set::current().suites.get(name).cloned().unwrap_or_default();
        }
        vec![root.to_string()]
    }
}

pub fn read(root: &Path) -> Held {
    let path = root.join("plumb.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Held::default();
    };
    parse(&text)
}

pub fn parse(text: &str) -> Held {
    let doc: toml::Table = match text.parse() {
        Ok(doc) => doc,
        Err(error) => return refuse(format!("cannot parse plumb.toml: {error}")),
    };
    let Some(seat) = doc.get("workflow").and_then(|held| held.get("hash")) else {
        return Held::default();
    };
    let Some(seat) = seat.as_table() else {
        return refuse("workflow.hash must be a table".to_string());
    };
    let mut keys: Vec<Key> = Vec::new();
    if let Err(error) = walk(seat, &mut Vec::new(), &mut keys) {
        return refuse(error);
    }
    for key in &keys {
        let lane = key.segments.first().cloned().unwrap_or_default();
        if !set::current().lanes.contains(&lane) {
            return refuse(format!("workflow.hash names no lane called {lane}"));
        }
        if key.roots.is_empty() {
            return refuse(format!("{} declares no path", key.name()));
        }
    }
    match resolve(&keys) {
        Ok(found) => {
            for (key, paths) in keys.iter_mut().zip(found) {
                key.paths = paths;
            }
            Held {
                keys,
                refusal: None,
            }
        }
        Err(error) => refuse(error),
    }
}

pub fn inferred(cargo: bool, pnpm: bool, plumb: bool, ectropy: bool) -> Held {
    let mut keys = Vec::new();
    if cargo {
        keys.push(key("rust", "suite://cargo"));
        keys.push(key("test", "suite://cargo"));
    }
    if pnpm {
        keys.push(key("web", "suite://pnpm"));
    }
    if plumb {
        keys.push(key("plumb", "*"));
    }
    if ectropy {
        keys.push(key("ectropy", "*"));
    }
    match resolve(&keys) {
        Ok(found) => {
            for (key, paths) in keys.iter_mut().zip(found) {
                key.paths = paths;
            }
            Held {
                keys,
                refusal: None,
            }
        }
        Err(error) => refuse(error),
    }
}

fn key(name: &str, root: &str) -> Key {
    Key {
        segments: vec!["guard".to_string(), name.to_string()],
        roots: vec![root.to_string()],
        paths: Vec::new(),
    }
}

fn resolve(keys: &[Key]) -> Result<Vec<Vec<String>>, String> {
    let index: BTreeMap<String, usize> = keys
        .iter()
        .enumerate()
        .map(|(at, key)| (key.name(), at))
        .collect();
    let mut found = Vec::new();
    for at in 0..keys.len() {
        let mut seen = Vec::new();
        found.push(gather(keys, &index, at, &mut seen)?);
    }
    Ok(found)
}

fn gather(
    keys: &[Key],
    index: &BTreeMap<String, usize>,
    at: usize,
    seen: &mut Vec<usize>,
) -> Result<Vec<String>, String> {
    if seen.contains(&at) {
        return Err(format!("{} takes part in a key cycle", keys[at].name()));
    }
    seen.push(at);
    let mut held = BTreeSet::new();
    for root in &keys[at].roots {
        if let Some(name) = root.strip_prefix("key://") {
            let next = index
                .get(name)
                .ok_or_else(|| format!("{} names no key called {name}", keys[at].name()))?;
            held.extend(gather(keys, index, *next, seen)?);
            continue;
        }
        if let Some(name) = root.strip_prefix("suite://") {
            held.extend(suite(name, &keys[at].name())?);
            continue;
        }
        held.insert(root.clone());
    }
    seen.pop();
    Ok(held.into_iter().collect())
}

fn walk(seat: &toml::Table, segments: &mut Vec<String>, keys: &mut Vec<Key>) -> Result<(), String> {
    for (name, value) in seat {
        segments.push(name.clone());
        let held = match value {
            toml::Value::Table(inner) => walk(inner, segments, keys),
            toml::Value::Array(list) => leaf(list, name).map(|roots| {
                keys.push(Key {
                    segments: segments.clone(),
                    roots,
                    paths: Vec::new(),
                });
            }),
            _ => Err(format!("{name} must hold a table or a list of paths")),
        };
        held?;
        segments.pop();
    }
    Ok(())
}

fn leaf(list: &[toml::Value], name: &str) -> Result<Vec<String>, String> {
    list.iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("{name} declares a path that is not text"))
        })
        .collect()
}

fn suite(name: &str, key: &str) -> Result<Vec<String>, String> {
    set::current()
        .suites
        .get(name)
        .cloned()
        .ok_or_else(|| format!("{key} names no suite called {name}"))
}

fn refuse(message: String) -> Held {
    Held {
        keys: Vec::new(),
        refusal: Some(message),
    }
}
