use crate::shape::depot::sha;
use crate::shape::layout::Declared;
use plumb::snapshot::Snapshot;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Faces<'a>(pub &'a Snapshot);

impl Faces<'_> {
    pub(crate) fn taken(&self, declared: &Declared, held: &[String]) -> BTreeMap<String, String> {
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

    pub(crate) fn members(&self, container: &str) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        for entry in self.0.seat(container) {
            let rest = entry.path().trim_start_matches(container).trim_matches('/');
            if let Some((head, _)) = rest.split_once('/') {
                found.insert(format!("{container}/{head}"));
            }
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
}

pub(crate) fn authority(faces: &BTreeMap<String, String>) -> String {
    let mut body = String::new();
    for (name, held) in faces {
        body.push_str(name);
        body.push(' ');
        body.push_str(held);
        body.push('\n');
    }
    sha(body.as_bytes())
}

fn seats(declared: &Declared) -> String {
    let mut held = declared
        .seats
        .iter()
        .filter(|seat| !seat.retired)
        .map(|seat| seat.path.clone())
        .collect::<Vec<_>>();
    held.extend(
        declared
            .groups
            .iter()
            .filter(|group| !group.retired)
            .flat_map(|group| group.names.clone()),
    );
    held.sort();
    held.join("\n")
}
