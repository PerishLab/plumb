use plumb::snapshot::Snapshot;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const FORMAT: u32 = 1;
pub const LEAF: &str = "plumb.toml";
pub const POINTER: &str = "metadata.json";
pub const ROOTS: [(&str, &str); 3] = [
    ("crates/cli/rules", "rules"),
    ("crates/lib/rules", "rules"),
    ("crates/cli/assets", "assets"),
];

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: Schema,
    pub metadata: Metadata,
    #[serde(default, rename = "object")]
    pub objects: Vec<Object>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Schema {
    pub format: u32,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub version: String,
    pub source: String,
    pub channel: String,
    pub commit: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Object {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

pub struct Plan {
    pub manifest: Manifest,
    pub bodies: BTreeMap<String, Vec<u8>>,
}

pub type Held = (Vec<Object>, BTreeMap<String, Vec<u8>>);

pub fn inventory(snapshot: &Snapshot) -> Result<Held, String> {
    let mut bodies = BTreeMap::new();
    let mut objects = Vec::new();
    for (root, seat) in ROOTS {
        for entry in snapshot.seat(root) {
            let name = entry
                .path()
                .strip_prefix(root)
                .map(|rest| rest.trim_start_matches('/'))
                .ok_or_else(|| format!("depot root {root} does not hold {}", entry.path()))?;
            let path = format!("{seat}/{name}");
            if bodies.contains_key(&path) {
                return Err(format!("depot object {path} is claimed twice"));
            }
            objects.push(Object {
                path: path.clone(),
                sha256: sha(entry.bytes()),
                size: entry.bytes().len() as u64,
            });
            bodies.insert(path, entry.bytes().to_vec());
        }
    }
    objects.sort();
    Ok((objects, bodies))
}

impl Plan {
    pub fn gather(snapshot: &Snapshot, metadata: Metadata, schema: Schema) -> Result<Self, String> {
        for (root, _) in ROOTS {
            if snapshot.seat(root).is_empty() {
                return Err(format!("depot root {root} records no object"));
            }
        }
        let (objects, bodies) = inventory(snapshot)?;
        Ok(Self {
            manifest: Manifest {
                schema,
                metadata,
                objects,
            },
            bodies,
        })
    }
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, String> {
        let held: Self = toml::from_str(text)
            .map_err(|error| format!("cannot parse a depot manifest: {error}"))?;
        if held.schema.format != FORMAT {
            return Err(format!(
                "depot manifest format must be {FORMAT}, got {}",
                held.schema.format
            ));
        }
        Ok(held)
    }

    pub fn encode(&self) -> Result<String, String> {
        toml::to_string(self).map_err(|error| format!("cannot encode a depot manifest: {error}"))
    }

    pub fn verify(&self, path: &str, bytes: &[u8]) -> Result<(), String> {
        let object = self
            .objects
            .iter()
            .find(|held| held.path == path)
            .ok_or_else(|| format!("depot manifest names no object at {path}"))?;
        if object.sha256 != sha(bytes) || object.size != bytes.len() as u64 {
            return Err(format!("depot object drift: {path}"));
        }
        Ok(())
    }
}

pub fn sha(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pointer {
    pub format: u32,
    pub product: String,
    pub channel: String,
    pub version: String,
    pub source: String,
    pub commit: String,
}

impl Pointer {
    pub fn new(metadata: &Metadata, product: &str) -> Self {
        Self {
            format: FORMAT,
            product: product.to_string(),
            channel: metadata.channel.clone(),
            version: metadata.version.clone(),
            source: metadata.source.clone(),
            commit: metadata.commit.clone(),
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let held: Self = serde_json::from_str(text)
            .map_err(|error| format!("cannot parse a depot pointer: {error}"))?;
        if held.format != FORMAT {
            return Err(format!(
                "depot pointer format must be {FORMAT}, got {}",
                held.format
            ));
        }
        Ok(held)
    }

    pub fn encode(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|error| format!("cannot encode a depot pointer: {error}"))
    }
}

pub fn versions(channel: &str, mark: &str) -> String {
    format!("channels/{channel}/versions/{mark}")
}

pub fn latest(channel: &str) -> String {
    format!("channels/{channel}/latest/{POINTER}")
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Notes {
    pub format: u32,
    pub version: String,
    pub commit: String,
    #[serde(default, rename = "object")]
    pub objects: Vec<Object>,
}

pub struct Batch {
    pub notes: Notes,
    pub bodies: BTreeMap<String, Vec<u8>>,
}

impl Batch {
    pub fn gather(source: &Path, version: &str, commit: &str) -> Result<Self, String> {
        let mut bodies = BTreeMap::new();
        let mut objects = Vec::new();
        for path in walk(source)? {
            let name = path
                .strip_prefix(source)
                .map_err(|error| {
                    format!(
                        "{} is outside {}: {error}",
                        path.display(),
                        source.display()
                    )
                })?
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = std::fs::read(&path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            objects.push(Object {
                path: name.clone(),
                sha256: sha(&bytes),
                size: bytes.len() as u64,
            });
            bodies.insert(name, bytes);
        }
        if objects.is_empty() {
            return Err(format!("{} holds no release note", source.display()));
        }
        objects.sort();
        Ok(Self {
            notes: Notes {
                format: FORMAT,
                version: version.to_string(),
                commit: commit.to_string(),
                objects,
            },
            bodies,
        })
    }
}

impl Notes {
    pub fn parse(text: &str) -> Result<Self, String> {
        let held: Self =
            toml::from_str(text).map_err(|error| format!("cannot parse release notes: {error}"))?;
        if held.format != FORMAT {
            return Err(format!(
                "release notes format must be {FORMAT}, got {}",
                held.format
            ));
        }
        Ok(held)
    }

    pub fn encode(&self) -> Result<String, String> {
        toml::to_string(self).map_err(|error| format!("cannot encode release notes: {error}"))
    }
}

pub fn changelog(version: &str) -> String {
    format!("changelog/{version}")
}

fn walk(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    let listed = std::fs::read_dir(root)
        .map_err(|error| format!("cannot read {}: {error}", root.display()))?;
    for entry in listed {
        let path = entry
            .map_err(|error| format!("cannot read {}: {error}", root.display()))?
            .path();
        if path.is_dir() {
            found.extend(walk(&path)?);
        } else {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}
