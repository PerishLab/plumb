use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

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
    pub origin: Option<Origin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<Recovery>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Recovery {
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
pub enum Origin {
    Stable {},
    #[serde(rename = "exact-release")]
    Exact {
        channel: String,
        #[serde(rename = "releaseVersion")]
        version: String,
        url: String,
        sha256: String,
    },
    #[serde(rename = "source-built")]
    Source {
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

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum Provenance {
    #[default]
    Native,
    #[serde(rename = "legacy-adopted")]
    Adopted {
        tag: String,
        cargo: Vec<String>,
        npm: Vec<String>,
    },
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
    #[serde(default)]
    pub provenance: Provenance,
    pub artifacts: BTreeMap<String, Remote>,
    pub managers: BTreeMap<String, Remote>,
    #[serde(default, rename = "changelog", skip_serializing)]
    pub legacy: Option<crate::command::changelog::Proof>,
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
