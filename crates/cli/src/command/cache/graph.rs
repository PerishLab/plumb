use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(super) struct Unit {
    pub fresh: u64,
    deps: Vec<String>,
}

pub(super) struct Profile {
    pub path: PathBuf,
    pub name: String,
    pub units: BTreeMap<String, Unit>,
    prints: BTreeMap<String, String>,
}

enum Kind {
    Manifest,
    Print,
    Other,
}

impl Kind {
    fn of(label: &str) -> Self {
        if label.ends_with(".json") {
            Self::Manifest
        } else if label.contains('.') || label.starts_with("dep-") || label.starts_with("output-") {
            Self::Other
        } else {
            Self::Print
        }
    }
}

pub(super) fn profiles(bucket: &Path) -> Result<Vec<Profile>, String> {
    let mut found = Vec::new();
    for entry in Node(bucket).listing()? {
        if entry.join(".fingerprint").is_dir() {
            found.push(Profile::read(&entry)?);
        }
    }
    Ok(found)
}

impl Profile {
    fn read(path: &Path) -> Result<Self, String> {
        let mut profile = Self {
            path: path.to_path_buf(),
            name: Node(path).name(),
            units: BTreeMap::new(),
            prints: BTreeMap::new(),
        };
        for unit in Node(&path.join(".fingerprint")).listing()? {
            profile.absorb(&unit)?;
        }
        Ok(profile)
    }

    fn absorb(&mut self, unit: &Path) -> Result<(), String> {
        let key = Node(unit).name();
        let mut held = Unit {
            fresh: 0,
            deps: Vec::new(),
        };
        for file in Node(unit).listing()? {
            held.fresh = held.fresh.max(Node(&file).stamp());
            match Kind::of(&Node(&file).name()) {
                Kind::Manifest => held.deps.extend(dependencies(&file)),
                Kind::Print => self.remember(&file, &key),
                Kind::Other => {}
            }
        }
        self.units.insert(key, held);
        Ok(())
    }

    fn remember(&mut self, file: &Path, key: &str) {
        let print = std::fs::read_to_string(file).unwrap_or_default();
        if !print.trim().is_empty() {
            self.prints
                .insert(print.trim().to_string(), key.to_string());
        }
    }

    pub fn closure(&self, since: u64) -> BTreeSet<String> {
        let mut live = BTreeSet::new();
        let mut pending: Vec<&String> = self
            .units
            .iter()
            .filter(|(_, unit)| unit.fresh >= since)
            .map(|(key, _)| key)
            .collect();
        while let Some(key) = pending.pop() {
            if !live.insert(key.clone()) {
                continue;
            }
            for print in &self.units[key].deps {
                if let Some(next) = self.prints.get(print) {
                    pending.push(next);
                }
            }
        }
        live
    }

    pub fn files(&self, key: &str) -> Result<Vec<PathBuf>, String> {
        let mut found = vec![
            self.path.join(".fingerprint").join(key),
            self.path.join("build").join(key),
        ];
        let hash = suffix(key);
        for entry in Node(&self.path.join("deps")).listing()? {
            if hash.is_some() && suffix(&Node(&entry).name()) == hash {
                found.push(entry);
            }
        }
        found.retain(|path| std::fs::symlink_metadata(path).is_ok());
        Ok(found)
    }

    pub fn orphans(&self) -> Result<Vec<PathBuf>, String> {
        let owned: BTreeSet<Option<String>> = self.units.keys().map(|key| suffix(key)).collect();
        let mut found = Vec::new();
        for seat in ["deps", "build"] {
            for entry in Node(&self.path.join(seat)).listing()? {
                let hash = suffix(&Node(&entry).name());
                if hash.is_some() && !owned.contains(&hash) {
                    found.push(entry);
                }
            }
        }
        Ok(found)
    }

    pub fn incremental(&self) -> Result<Vec<(String, PathBuf, u64)>, String> {
        let mut found = Vec::new();
        for entry in Node(&self.path.join("incremental")).listing()? {
            let fresh = Node(&entry).newest();
            let key = format!("incremental/{}", Node(&entry).name());
            found.push((key, entry, fresh));
        }
        Ok(found)
    }
}

fn dependencies(file: &Path) -> Vec<String> {
    let Ok(bytes) = std::fs::read(file) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Vec::new();
    };
    value["deps"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|dep| dep[3].as_u64())
        .map(|print| {
            print
                .to_le_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect()
        })
        .collect()
}

fn suffix(entry: &str) -> Option<String> {
    let (_, tail) = entry.rsplit_once('-')?;
    let hash = tail.split('.').next()?;
    (hash.len() == 16 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| hash.to_string())
}

pub(super) struct Node<'a>(pub &'a Path);

impl Node<'_> {
    pub fn listing(&self) -> Result<Vec<PathBuf>, String> {
        let path = self.0;
        match std::fs::read_dir(path) {
            Ok(entries) => entries
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<Result<_, _>>()
                .map_err(|error| format!("cannot read {}: {error}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(format!("cannot read {}: {error}", path.display())),
        }
    }

    pub fn name(&self) -> String {
        self.0
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub fn stamp(&self) -> u64 {
        std::fs::symlink_metadata(self.0)
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_secs())
    }

    fn newest(&self) -> u64 {
        let mut fresh = self.stamp();
        for entry in self.listing().unwrap_or_default() {
            let node = Node(&entry);
            fresh = fresh.max(if entry.is_dir() {
                node.newest()
            } else {
                node.stamp()
            });
        }
        fresh
    }

    pub fn size(&self) -> u64 {
        let Ok(metadata) = std::fs::symlink_metadata(self.0) else {
            return 0;
        };
        if !metadata.is_dir() {
            return metadata.len();
        }
        self.listing()
            .unwrap_or_default()
            .iter()
            .map(|entry| Node(entry).size())
            .sum()
    }
}
