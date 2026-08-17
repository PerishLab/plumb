use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Remote {
    pub name: String,
    pub mime: String,
    pub sha256: String,
    pub size: u64,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Generator {
    pub version: String,
    pub template: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<GeneratorOrigin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<RecoveryIdentity>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RecoveryIdentity {
    pub repository: String,
    pub authority: String,
    pub beta: String,
    pub stable: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum GeneratorOrigin {
    Stable {},
    ExactRelease {
        channel: String,
        #[serde(rename = "releaseVersion")]
        version: String,
        url: String,
        sha256: String,
    },
    SourceBuilt {
        repository: String,
        commit: String,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Promotion {
    pub seal: Box<Seal>,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Held {
    pub label: String,
    pub ecosystem: String,
    pub lock: String,
    pub resolution: String,
    pub behind: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Unread {
    pub label: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Radius {
    pub schema: String,
    pub product: String,
    pub candidate: String,
    pub seats: Vec<Held>,
    pub behind: usize,
    pub blind: Vec<Unread>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Seal {
    pub schema: u32,
    pub product: String,
    pub channel: String,
    #[serde(rename = "releaseVersion")]
    pub version: String,
    pub commit: String,
    pub url: String,
    pub generator: Generator,
    pub artifacts: BTreeMap<String, Remote>,
    pub managers: BTreeMap<String, Remote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changelog: Option<crate::shape::changelog::Proof>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<Promotion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<Radius>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub inputs: BTreeMap<String, Input>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Input {
    pub hash: String,
    pub since: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Local {
    pub source: String,
    pub key: String,
    pub remote: Remote,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Pointer {
    pub schema: u32,
    pub product: String,
    pub channel: String,
    #[serde(rename = "releaseVersion")]
    pub version: String,
    pub commit: String,
    pub seal: Remote,
    pub managers: BTreeMap<String, Remote>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Capsule {
    pub schema: u32,
    pub product: String,
    pub channel: String,
    #[serde(rename = "releaseVersion")]
    pub version: String,
    pub authority: String,
    pub objects: Vec<Local>,
    pub seal: Local,
    #[serde(default)]
    pub roots: Vec<Local>,
    pub pointer: Option<Local>,
}

impl Capsule {
    pub fn read(path: &Path) -> Result<(Self, PathBuf), String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let held: Self = serde_json::from_str(&text)
            .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
        if held.schema != 1 {
            return Err(format!("capsule schema must be 1, got {}", held.schema));
        }
        let root = path
            .parent()
            .ok_or_else(|| format!("capsule has no parent: {}", path.display()))?
            .to_path_buf();
        for object in held
            .objects
            .iter()
            .chain(held.roots.iter())
            .chain(std::iter::once(&held.seal))
            .chain(held.pointer.iter())
        {
            object.verify(&root)?;
        }
        Ok((held, root))
    }
}

impl Local {
    fn verify(&self, root: &Path) -> Result<(), String> {
        let path = root.join(&self.source);
        let (digest, size) = digest(&path)?;
        if digest != self.remote.sha256 || size != self.remote.size {
            return Err(format!("local capsule object drift: {}", path.display()));
        }
        Ok(())
    }
}

pub fn digest(path: &Path) -> Result<(String, u64), String> {
    let bytes =
        std::fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Ok((sha(&bytes), bytes.len() as u64))
}

pub fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let mut text = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    text.push('\n');
    std::fs::write(path, text).map_err(|error| format!("cannot write {}: {error}", path.display()))
}
