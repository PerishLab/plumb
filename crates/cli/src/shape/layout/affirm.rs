use super::{Declared, Group, Seat};
use crate::dispatch::depot::record::sha;
use plumb::snapshot::Snapshot;
use std::collections::BTreeMap;
use std::path::Path;

pub struct Faces<'a>(pub &'a Snapshot);

impl Faces<'_> {
    pub fn taken(&self, declared: &Declared, held: &[String]) -> BTreeMap<String, String> {
        let mut found = BTreeMap::new();
        for name in held {
            let body = match name.as_str() {
                "declaration" => self.declaration(),
                "seat" => seats(declared),
                "lane" => self.lanes(),
                _ => continue,
            };
            found.insert(name.clone(), sha(body.as_bytes()));
        }
        found
    }

    fn declaration(&self) -> String {
        self.0
            .entries()
            .iter()
            .find(|entry| entry.path() == "plumb.toml")
            .map(|entry| String::from_utf8_lossy(entry.bytes()).to_string())
            .unwrap_or_default()
    }

    fn lanes(&self) -> String {
        let mut held = self
            .0
            .entries()
            .iter()
            .filter_map(|entry| entry.path().strip_prefix(".forgejo/workflows/"))
            .filter_map(|name| name.strip_suffix(".yml"))
            .map(str::to_string)
            .collect::<Vec<_>>();
        held.sort();
        held.join("\n")
    }

    fn members(&self, container: &str) -> std::collections::BTreeSet<String> {
        let mut found = std::collections::BTreeSet::new();
        for entry in self.0.seat(container) {
            let rest = entry.path().trim_start_matches(container).trim_matches('/');
            if let Some((head, _)) = rest.split_once('/') {
                found.insert(format!("{container}/{head}"));
            }
        }
        found
    }

    fn stamp(
        &self,
        declared: &Declared,
        member: &super::rule::Member,
        held: (&str, &[String]),
    ) -> Vec<Owed> {
        let (rule, targets) = held;
        if member.affirms.is_empty() {
            return Vec::new();
        }
        let authority = authority(&self.taken(declared, &member.affirms));
        targets
            .iter()
            .map(|target| Owed {
                target: target.clone(),
                rule: rule.to_string(),
                authority: authority.clone(),
            })
            .collect()
    }
}

pub const SEAT: &str = ".plumb/affirmed.toml";

pub struct Held(BTreeMap<String, String>);

impl Held {
    pub fn read(root: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(root.join(SEAT)) else {
            return Self(BTreeMap::new());
        };
        let Ok(doc) = text.parse::<toml::Table>() else {
            return Self(BTreeMap::new());
        };
        let mut held = BTreeMap::new();
        let listed = doc
            .get("record")
            .and_then(toml::Value::as_array)
            .cloned()
            .unwrap_or_default();
        for entry in listed {
            let target = entry.get("target").and_then(toml::Value::as_str);
            let authority = entry.get("authority").and_then(toml::Value::as_str);
            if let (Some(target), Some(authority)) = (target, authority) {
                held.insert(target.to_string(), authority.to_string());
            }
        }
        Self(held)
    }

    pub fn authority(&self, target: &str) -> Option<&str> {
        self.0.get(target).map(String::as_str)
    }
}

pub fn authority(faces: &BTreeMap<String, String>) -> String {
    let mut body = String::new();
    for (name, held) in faces {
        body.push_str(name);
        body.push(' ');
        body.push_str(held);
        body.push('\n');
    }
    sha(body.as_bytes())
}

pub fn record(target: &str, rule: &str, authority: &str) -> String {
    format!("[[record]]\ntarget = \"{target}\"\nrule = \"{rule}\"\nauthority = \"{authority}\"\n")
}

fn seats(declared: &Declared) -> String {
    let mut held = declared
        .seats
        .iter()
        .filter(|seat| !seat.retired)
        .map(|seat: &Seat| seat.path.clone())
        .collect::<Vec<_>>();
    held.extend(
        declared
            .groups
            .iter()
            .filter(|group| !group.retired)
            .flat_map(|group: &Group| group.names.clone()),
    );
    held.sort();
    held.join("\n")
}

pub struct Owed {
    pub target: String,
    pub rule: String,
    pub authority: String,
}

pub fn owed(root: &Path) -> Result<Vec<Owed>, String> {
    let snapshot = Snapshot::read(root).map_err(|error| error.to_string())?;
    let super::Held::Stated(declared) = super::stated(root) else {
        return Err("no layout is declared".to_string());
    };
    let mut found = Vec::new();
    for seat in declared.seats.iter().filter(|seat| !seat.retired) {
        for held in &seat.rule {
            let Ok(member) = super::rule::parse(held).and_then(|held| super::rule::member(&held))
            else {
                continue;
            };
            let (Some(container), Some(leaf)) = (seat.container(), member.leaf.as_deref()) else {
                continue;
            };
            let targets = Faces(&snapshot)
                .members(container)
                .into_iter()
                .map(|name| format!("{name}/{leaf}"))
                .collect::<Vec<_>>();
            found.extend(Faces(&snapshot).stamp(&declared, &member, (held, &targets)));
        }
    }
    for group in declared.groups.iter().filter(|group| !group.retired) {
        for held in &group.rule {
            let Ok(member) = super::rule::parse(held).and_then(|held| super::rule::member(&held))
            else {
                continue;
            };
            found.extend(Faces(&snapshot).stamp(&declared, &member, (held, &group.names)));
        }
    }
    Ok(found)
}

pub fn write(root: &Path, owed: &[Owed]) -> Result<(), String> {
    let mut body = String::new();
    for held in owed {
        body.push_str(&record(&held.target, &held.rule, &held.authority));
        body.push('\n');
    }
    let seat = root.join(SEAT);
    if let Some(base) = seat.parent() {
        std::fs::create_dir_all(base).map_err(|error| error.to_string())?;
    }
    std::fs::write(&seat, body).map_err(|error| error.to_string())
}
