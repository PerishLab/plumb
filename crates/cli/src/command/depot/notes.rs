use crate::shape::depot::{FORMAT, Object, sha};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const FLOOR: &str = "v0.0.0";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Notes {
    pub format: u32,
    #[serde(default = "floor")]
    pub floor: String,
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
                floor: FLOOR.to_string(),
                version: version.to_string(),
                commit: commit.to_string(),
                objects,
            },
            bodies,
        })
    }
}

fn floor() -> String {
    FLOOR.to_string()
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
        held.supported()?;
        Ok(held)
    }

    fn supported(&self) -> Result<(), String> {
        let running = plumb::version!("PLUMB");
        let (Ok(held), Ok(least)) = (
            semver::Version::parse(running.trim_start_matches('v')),
            semver::Version::parse(self.floor.trim_start_matches('v')),
        ) else {
            return Err(format!(
                "cannot compare a running {running} with a note floor of {}",
                self.floor
            ));
        };
        if held >= least {
            return Ok(());
        }
        Err(format!(
            "release notes for {} declare a floor of {}, above the running {running}",
            self.version, self.floor
        ))
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
