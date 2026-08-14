use semver::Version;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Ecosystem {
    Cargo,
}

impl Ecosystem {
    pub fn name(self) -> &'static str {
        match self {
            Self::Cargo => "cargo",
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Dependency {
    pub ecosystem: Ecosystem,
    pub name: String,
    pub requirement: String,
    pub pinned: bool,
    pub resolution: String,
    pub latest: Option<String>,
    pub seat: String,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Verdict {
    Stale { resolution: String, latest: String },
    Unread(String),
}

#[derive(Deserialize)]
struct Cargo {
    vers: String,
    #[serde(default)]
    yanked: bool,
}

pub fn judge(dependency: &Dependency) -> Vec<Verdict> {
    let mut found = Vec::new();
    let Some(latest) = &dependency.latest else {
        return found;
    };
    let resolution = match Version::parse(&dependency.resolution) {
        Ok(version) => version,
        Err(error) => {
            found.push(Verdict::Unread(format!(
                "lock resolution {}: {error}",
                dependency.resolution
            )));
            return found;
        }
    };
    let current = match Version::parse(latest) {
        Ok(version) => version,
        Err(error) => {
            found.push(Verdict::Unread(format!(
                "registry version {latest}: {error}"
            )));
            return found;
        }
    };
    if resolution != current {
        found.push(Verdict::Stale {
            resolution: resolution.to_string(),
            latest: current.to_string(),
        });
    }
    found
}

pub fn cargo(bytes: &[u8]) -> Result<String, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| format!("invalid UTF-8: {error}"))?;
    let mut latest = None;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let entry: Cargo =
            serde_json::from_str(line).map_err(|error| format!("invalid index entry: {error}"))?;
        let version = Version::parse(&entry.vers)
            .map_err(|error| format!("invalid index version {}: {error}", entry.vers))?;
        if entry.yanked || !version.pre.is_empty() {
            continue;
        }
        if latest.as_ref().is_none_or(|held| version > *held) {
            latest = Some(version);
        }
    }
    latest
        .map(|version| version.to_string())
        .ok_or_else(|| "registry exposes no stable version".into())
}

pub fn route(name: &str) -> String {
    let name = name.to_ascii_lowercase();
    match name.len() {
        1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{}", &name[0..1], name),
        _ => format!("{}/{}/{}", &name[0..2], &name[2..4], name),
    }
}

pub fn packages(document: &toml::Value, registry: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    walk(document, registry, &mut found);
    found
}

fn walk(value: &toml::Value, registry: &str, found: &mut BTreeSet<String>) {
    let Some(table) = value.as_table() else {
        return;
    };
    for (name, value) in table {
        if matches!(
            name.as_str(),
            "dependencies" | "dev-dependencies" | "build-dependencies"
        ) {
            collect(value, registry, found);
        } else if name == "workspace" || name == "target" {
            walk(value, registry, found);
        }
    }
}

fn collect(value: &toml::Value, registry: &str, found: &mut BTreeSet<String>) {
    let Some(table) = value.as_table() else {
        return;
    };
    for (name, value) in table {
        let Some(specification) = value.as_table() else {
            continue;
        };
        if specification.get("registry").and_then(toml::Value::as_str) != Some(registry) {
            continue;
        }
        found.insert(
            specification
                .get("package")
                .and_then(toml::Value::as_str)
                .unwrap_or(name)
                .to_string(),
        );
    }
}
