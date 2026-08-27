use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::{Object, sha};
use value::Value;

mod kind;
mod local;
mod value;

#[cfg(feature = "depot")]
pub mod media;

pub use kind::Kind;
pub use local::{FORMAT, LEAF, POINTER, local};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Seal {
    pub url: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub product: String,
    pub channel: String,
    pub version: String,
    pub commit: String,
    pub seal: Seal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub timestamp: String,
    pub commit: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: u32,
    pub source: String,
    pub derivative: Kind,
    pub release: Release,
    pub snapshot: Snapshot,
    #[serde(default, rename = "object")]
    pub objects: Vec<Object>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub sha256: String,
    pub size: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pointer {
    pub format: u32,
    pub source: String,
    pub derivative: Kind,
    pub release: Release,
    pub snapshot: Snapshot,
    pub manifest: Reference,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, String> {
        let held: Self = toml::from_str(text)
            .map_err(|error| format!("cannot parse a depot v2 manifest: {error}"))?;
        held.validate()?;
        Ok(held)
    }

    pub fn encode(&self) -> Result<String, String> {
        self.validate()?;
        toml::to_string(self).map_err(|error| format!("cannot encode a depot v2 manifest: {error}"))
    }

    pub fn verify(&self, path: &str, bytes: &[u8]) -> Result<(), String> {
        Value(path).anchored("depot object path")?;
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

    fn validate(&self) -> Result<(), String> {
        if self.format != FORMAT {
            return Err(format!(
                "depot manifest format must be {FORMAT}, got {}",
                self.format
            ));
        }
        Value(&self.source).source()?;
        self.release.validate()?;
        self.snapshot.validate()?;
        let mut paths = BTreeSet::new();
        for object in &self.objects {
            Value(&object.path).anchored("depot object path")?;
            Value(&object.sha256).digest("depot object sha256")?;
            if !paths.insert(&object.path) {
                return Err(format!("depot manifest repeats {}", object.path));
            }
        }
        Ok(())
    }
}

impl Pointer {
    pub fn new(manifest: &Manifest, bytes: &[u8]) -> Result<Self, String> {
        manifest.validate()?;
        Ok(Self {
            format: FORMAT,
            source: manifest.source.clone(),
            derivative: manifest.derivative,
            release: manifest.release.clone(),
            snapshot: manifest.snapshot.clone(),
            manifest: Reference {
                sha256: sha(bytes),
                size: bytes.len() as u64,
            },
        })
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let held: Self = serde_json::from_str(text)
            .map_err(|error| format!("cannot parse a depot v2 pointer: {error}"))?;
        held.validate()?;
        Ok(held)
    }

    pub fn encode(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self)
            .map_err(|error| format!("cannot encode a depot v2 pointer: {error}"))
    }

    pub fn bind(&self, manifest: &Manifest, bytes: &[u8]) -> Result<(), String> {
        manifest.validate()?;
        let reference = Reference {
            sha256: sha(bytes),
            size: bytes.len() as u64,
        };
        if self.identity() != manifest.identity() || self.manifest != reference {
            return Err(format!(
                "depot pointer {} does not bind its manifest",
                self.snapshot.timestamp
            ));
        }
        Ok(())
    }

    pub fn advance(&self, next: &Self) -> Result<bool, String> {
        if self == next {
            return Ok(false);
        }
        let standing = (
            &self.source,
            self.derivative,
            &self.release.product,
            &self.release.channel,
        );
        let proposed = (
            &next.source,
            next.derivative,
            &next.release.product,
            &next.release.channel,
        );
        if standing != proposed {
            return Err("standing depot pointer names another derivative".into());
        }
        if self.release == next.release && self.snapshot.timestamp >= next.snapshot.timestamp {
            return Err(format!(
                "depot latest would not advance from {} to {}",
                self.snapshot.timestamp, next.snapshot.timestamp
            ));
        }
        Ok(true)
    }

    fn identity(&self) -> Identity<'_> {
        Identity {
            source: &self.source,
            derivative: self.derivative,
            release: &self.release,
            snapshot: &self.snapshot,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.format != FORMAT {
            return Err(format!(
                "depot pointer format must be {FORMAT}, got {}",
                self.format
            ));
        }
        Value(&self.source).source()?;
        self.release.validate()?;
        self.snapshot.validate()?;
        Value(&self.manifest.sha256).digest("depot manifest sha256")
    }
}

#[derive(PartialEq)]
struct Identity<'a> {
    source: &'a str,
    derivative: Kind,
    release: &'a Release,
    snapshot: &'a Snapshot,
}

impl Manifest {
    fn identity(&self) -> Identity<'_> {
        Identity {
            source: &self.source,
            derivative: self.derivative,
            release: &self.release,
            snapshot: &self.snapshot,
        }
    }
}

impl Release {
    fn validate(&self) -> Result<(), String> {
        Value(&self.product).component("release product")?;
        Value(&self.channel).component("release channel")?;
        Value(&self.version).component("release version")?;
        semver::Version::parse(self.version.trim_start_matches('v'))
            .map_err(|error| format!("cannot parse release version {}: {error}", self.version))?;
        Value(&self.commit).commit("release commit")?;
        Value(&self.seal.url).source()?;
        Value(&self.seal.sha256).digest("release seal sha256")
    }
}

impl Snapshot {
    fn validate(&self) -> Result<(), String> {
        Value(&self.timestamp).component("snapshot timestamp")?;
        let bytes = self.timestamp.as_bytes();
        if bytes.len() != 16 {
            return Err(format!(
                "invalid depot snapshot timestamp: {}",
                self.timestamp
            ));
        }
        let digits = bytes[..8].iter().chain(&bytes[9..15]);
        if bytes[8] != b'T'
            || bytes[15] != b'Z'
            || !digits.cloned().all(|byte| byte.is_ascii_digit())
        {
            return Err(format!(
                "invalid depot snapshot timestamp: {}",
                self.timestamp
            ));
        }
        Value(&self.commit).commit("snapshot commit")
    }
}

pub fn snapshots(release: &Release, derivative: Kind, timestamp: &str) -> Result<String, String> {
    release.validate()?;
    Value(timestamp).component("snapshot timestamp")?;
    Ok(format!(
        "v2/products/{}/derivatives/{}/releases/{}/{}/snapshots/{timestamp}",
        release.product,
        derivative.label(),
        release.channel,
        release.version
    ))
}

pub fn latest(product: &str, derivative: Kind, channel: &str) -> Result<String, String> {
    Value(product).component("release product")?;
    Value(channel).component("release channel")?;
    Ok(format!(
        "v2/products/{product}/derivatives/{}/channels/{channel}/latest.json",
        derivative.label()
    ))
}
