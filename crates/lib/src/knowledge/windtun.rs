use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[cfg(feature = "windtun")]
#[path = "qualification/mod.rs"]
pub mod qualification;

pub const SCHEMA: &str = "plumb.windtun.declaration/v1";
pub const COMBINATION: &str = "plumb.windtun.declaration/v2";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Inspection {
    pub schema: &'static str,
    pub manifest: Option<String>,
    pub windtun: Option<Declaration>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    pub scenes: Scenes,
    pub cases: Vec<String>,
    pub image: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub services: BTreeMap<String, Service>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scenes {
    pub authority: String,
    pub product: String,
    pub channel: String,
}

pub fn parse(text: &str) -> Result<Option<Declaration>, String> {
    let document: toml::Table = text
        .parse()
        .map_err(|error| format!("invalid plumb.toml: {error}"))?;
    let legacy = match document.get("guard") {
        None => None,
        Some(value) => {
            let guard = value.as_table().ok_or("guard must be a table")?;
            if guard
                .keys()
                .any(|key| !matches!(key.as_str(), "seatrial" | "lock"))
            {
                return Err(
                    "guard declares only lock protection and the legacy Seatrial suite".into(),
                );
            }
            if guard.get("lock").is_some_and(|value| !value.is_table()) {
                return Err("guard.lock must be a table".into());
            }
            guard.get("seatrial")
        }
    };
    let value = match (document.get("windtun"), legacy) {
        (None, None) => return Ok(None),
        (Some(_), Some(_)) => {
            return Err("declare windtun once; remove legacy guard.seatrial".into());
        }
        (Some(value), None) | (None, Some(value)) => value,
    };
    let held: Declaration = value
        .clone()
        .try_into()
        .map_err(|error| format!("windtun: {error}"))?;
    held.validate()?;
    if document
        .get("identity")
        .and_then(|v| v.get("ns"))
        .and_then(|v| v.as_str())
        .is_some_and(|name| held.services.contains_key(name))
    {
        return Err("Windtun service cannot repeat the published target".into());
    }
    Ok(Some(held))
}

pub fn inspect(root: &Path) -> Result<Inspection, String> {
    if !root.is_dir() {
        return Err("Windtun inspection requires an existing repository directory".into());
    }
    let path = root.join("plumb.toml");
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("cannot inspect plumb.toml: {error}")),
    };
    let Some(metadata) = metadata else {
        return Ok(Inspection {
            schema: SCHEMA,
            manifest: None,
            windtun: None,
        });
    };
    if !metadata.file_type().is_file() {
        return Err("Windtun declaration requires a regular plumb.toml".into());
    }
    let raw = std::fs::read(path).map_err(|error| format!("cannot read plumb.toml: {error}"))?;
    let text = std::str::from_utf8(&raw).map_err(|_| "invalid declaration UTF-8")?;
    let windtun = parse(text)?;
    Ok(Inspection {
        schema: if windtun
            .as_ref()
            .is_some_and(|value| !value.services.is_empty())
        {
            COMBINATION
        } else {
            SCHEMA
        },
        manifest: Some(format!("{:x}", Sha256::digest(&raw))),
        windtun,
    })
}

pub fn declared(snapshot: &crate::snapshot::Snapshot) -> Result<Option<Declaration>, String> {
    let Some(entry) = snapshot
        .entries()
        .iter()
        .find(|entry| entry.path() == "plumb.toml")
    else {
        return Ok(None);
    };
    if !matches!(entry.mode(), "100644" | "100755") {
        return Err("Seatrial declaration requires a tracked regular plumb.toml".into());
    }
    parse(std::str::from_utf8(entry.bytes()).map_err(|_| "invalid declaration UTF-8")?)
}

impl Declaration {
    pub fn validate(&self) -> Result<(), String> {
        if self.services.len() > 4 {
            return Err("Windtun admits at most four explicit services".into());
        }
        for (name, service) in &self.services {
            service.validate(name)?;
        }
        let scenes = &self.scenes;
        let authority = &scenes.authority;
        crate::seat::release::Authority::new(authority)?;
        if authority.contains(['?', '#', '@']) {
            return Err("Seatrial scenes require a normalized public HTTPS authority".into());
        }
        if !name(&scenes.product)
            || !["stable", "alpha", "beta", "rc"].contains(&scenes.channel.as_str())
        {
            return Err(
                "Seatrial scenes require a product and channel, never a version pin".into(),
            );
        }
        let unique: BTreeSet<_> = self.cases.iter().collect();
        if self.cases.is_empty()
            || unique.len() != self.cases.len()
            || self.cases.iter().any(|case| !name(case))
        {
            return Err("Seatrial cases must be explicit nonempty unique names".into());
        }
        if self.image.is_empty() || self.image.chars().any(char::is_whitespace) {
            return Err("Seatrial requires a runtime image reference".into());
        }
        if self.image.starts_with("ghcr.io/perishlab/") {
            let tag = self.image.rsplit('/').next().unwrap_or_default();
            if self.image.contains('@')
                || tag
                    .split_once(':')
                    .is_some_and(|(_, tag)| !["stable", "alpha", "beta", "rc"].contains(&tag))
            {
                return Err(
                    "first-party Seatrial images follow a channel or the resolved product release"
                        .into(),
                );
            }
        } else if !self
            .image
            .rsplit_once("@sha256:")
            .is_some_and(|(_, digest)| {
                digest.len() == 64
                    && digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        {
            return Err("third-party Seatrial images require an immutable SHA-256 digest".into());
        }
        Ok(())
    }
}

fn name(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub authority: String,
    pub channel: String,
    pub placement: Placement,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub artifact: String,
    pub entry: String,
    pub paths: BTreeMap<String, String>,
}

impl Service {
    pub fn validate(&self, name: &str) -> Result<(), String> {
        if !self::name(name)
            || self.authority != format!("https://releases.{name}.perish.uk")
            || self.channel != "stable"
        {
            return Err("Windtun services require canonical first-party stable channels".into());
        }
        self.placement.validate()
    }
}

impl Placement {
    pub fn validate(&self) -> Result<(), String> {
        if self.artifact != "linux-x64-deb" || !name(&self.entry) || self.paths.len() != 1 {
            return Err("Windtun service placement requires one explicit Debian executable".into());
        }
        let path = self
            .paths
            .get(&self.entry)
            .ok_or("Windtun service placement lacks its exact primary entry")?;
        if path.is_empty() || path.len() > 256 {
            return Err("Windtun service path exceeds its nonempty byte bound".into());
        }
        if path
            .bytes()
            .any(|byte| !byte.is_ascii_graphic() || byte == b'\\')
            || path.split('/').any(|part| matches!(part, "" | "." | ".."))
        {
            return Err(
                "Windtun service placement requires a canonical package-relative path".into(),
            );
        }
        Ok(())
    }
}
