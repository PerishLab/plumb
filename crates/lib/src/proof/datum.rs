use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SCHEMA: u32 = 1;
pub const HOME: &str = ".plumb";
pub const SEAT: &str = ".plumb/releases";
pub const LEAF: &str = "datum.toml";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Datum {
    pub schema: u32,
    pub version: String,
    #[serde(default, rename = "answer")]
    pub answers: Vec<Answer>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    pub ecosystem: String,
    pub name: String,
    pub latest: String,
}

impl Datum {
    pub fn new(version: &str, answers: Vec<Answer>) -> Self {
        let mut answers = answers;
        answers.sort();
        answers.dedup();
        Self {
            schema: SCHEMA,
            version: version.to_string(),
            answers,
        }
    }

    pub fn latest(&self, ecosystem: &str, name: &str) -> Option<&str> {
        self.answers
            .iter()
            .find(|answer| answer.ecosystem == ecosystem && answer.name == name)
            .map(|answer| answer.latest.as_str())
    }

    pub fn encode(&self) -> Result<String, String> {
        toml::to_string(self).map_err(|error| format!("cannot encode datum: {error}"))
    }
}

pub fn leaf(version: &str) -> String {
    format!("{SEAT}/{version}/{LEAF}")
}

pub struct Tree<'a>(pub &'a Path);

impl Tree<'_> {
    pub fn seat(&self, version: &str) -> PathBuf {
        self.0.join(leaf(version))
    }

    pub fn read(&self, version: &str) -> Result<Option<Datum>, String> {
        let path = self.seat(version);
        if !path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        decode(version, &bytes).map(Some)
    }

    pub fn line(&self, declared: &str) -> Option<String> {
        let declared = declared.trim();
        if !declared.is_empty() {
            return Some(named(declared));
        }
        self.branch()
            .filter(|name| name.starts_with("release/"))
            .map(|name| named(&name))
    }

    fn branch(&self) -> Option<String> {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(["branch", "--show-current"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!name.is_empty()).then_some(name)
    }
}

pub fn decode(version: &str, bytes: &[u8]) -> Result<Datum, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("cannot read {}: {error}", leaf(version)))?;
    let datum: Datum =
        toml::from_str(text).map_err(|error| format!("cannot parse {}: {error}", leaf(version)))?;
    if datum.schema != SCHEMA {
        return Err(format!(
            "{} carries schema {}, this Plumb reads {SCHEMA}",
            leaf(version),
            datum.schema
        ));
    }
    if datum.version != version {
        return Err(format!(
            "{} names release {}, not {version}",
            leaf(version),
            datum.version
        ));
    }
    Ok(datum)
}

fn named(reference: &str) -> String {
    let reference = reference
        .trim_start_matches("refs/heads/")
        .trim_start_matches("release/");
    semver::Version::parse(reference.strip_prefix('v').unwrap_or(reference)).map_or_else(
        |_| reference.to_string(),
        |version| format!("v{}.{}.{}", version.major, version.minor, version.patch),
    )
}
