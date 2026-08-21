use crate::catalog::set::RULES;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

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
            return RULES.suites.get(name).cloned().unwrap_or_default();
        }
        vec![root.to_string()]
    }
}

pub struct Tree(BTreeMap<String, String>);

impl Tree {
    pub fn read(root: &Path, rev: Option<&str>) -> Result<Self, String> {
        let listed = match rev {
            Some(rev) => listing(root, &["ls-tree", "-r", "-z", rev])?,
            None => listing(root, &["ls-files", "--stage", "-z"])?,
        };
        let mut held = BTreeMap::new();
        for record in listed.split('\0').filter(|record| !record.is_empty()) {
            let (meta, path) = record
                .split_once('\t')
                .ok_or_else(|| "git emitted a malformed entry".to_string())?;
            let mut fields = meta.split(' ');
            let mode = fields.next().unwrap_or_default();
            let oid = match rev {
                Some(_) => fields.nth(1).unwrap_or_default(),
                None => fields.next().unwrap_or_default(),
            };
            if mode.is_empty() || oid.is_empty() {
                return Err("git emitted a malformed entry".to_string());
            }
            held.insert(path.to_string(), format!("{mode} {oid}"));
        }
        Ok(Self(held))
    }

    fn under(&self, roots: &[String]) -> Vec<(&String, &String)> {
        self.0
            .iter()
            .filter(|(path, _)| roots.iter().any(|root| covers(root, path)))
            .collect()
    }

    pub fn digest(&self, key: &Key) -> String {
        let mut sponge = Sha256::new();
        sponge.update(key.name().as_bytes());
        sponge.update([0]);
        for root in &key.roots {
            sponge.update(root.as_bytes());
            sponge.update([0]);
        }
        sponge.update([1]);
        sponge.update(RULES.release.forge.as_bytes());
        sponge.update([0]);
        for (path, meta) in self.under(&key.paths) {
            sponge.update(path.as_bytes());
            sponge.update([0]);
            sponge.update(meta.as_bytes());
            sponge.update([0]);
        }
        format!("{:x}", sponge.finalize())
    }

    pub fn covered(&self, key: &Key) -> usize {
        self.under(&key.paths).len()
    }
}

fn covers(root: &str, path: &str) -> bool {
    root == "*" || path == root || path.starts_with(&format!("{root}/"))
}

pub fn read(root: &Path) -> Held {
    let path = root.join("plumb.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Held::default();
    };
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
        if !RULES.lanes.contains(&lane) {
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
    RULES
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

pub fn history(root: &Path, rev: &str) -> Result<Vec<String>, String> {
    let range = format!("{rev}..HEAD");
    let listed = listing(root, &["rev-list", "--reverse", &range])?;
    Ok(listed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

fn listing(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| format!("cannot execute git: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    String::from_utf8(output.stdout).map_err(|_| "git emitted non-UTF-8 output".to_string())
}
