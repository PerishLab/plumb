use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

mod rules;
pub mod v2;
pub mod v3;
pub use rules::Rules;

pub const FORMAT: u32 = 1;
pub const LEAF: &str = "plumb.toml";
pub const POINTER: &str = "metadata.json";

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

pub struct Seat {
    base: PathBuf,
    manifest: Manifest,
}

pub fn rules() -> Result<&'static Rules, String> {
    rules::held()
}

impl Seat {
    pub fn open() -> Result<Self, String> {
        Self::at(&root(&PathBuf::new())?)
    }

    pub fn at(root: &Path) -> Result<Self, String> {
        let marker = root.join(POINTER);
        let text = std::fs::read_to_string(&marker)
            .map_err(|error| format!("cannot read {}: {error}", marker.display()))?;
        let pointer = Pointer::parse(&text)?;
        let base = root.join(&pointer.version);
        let path = base.join(LEAF);
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let manifest = Manifest::parse(&text)?;
        manifest.bound(&pointer)?;
        Ok(Self { base, manifest })
    }

    pub fn read(&self, path: &str) -> Result<String, String> {
        anchored(path)?;
        let file = self.base.join(path);
        let bytes = std::fs::read(&file)
            .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
        self.manifest.verify(path, &bytes)?;
        String::from_utf8(bytes).map_err(|error| format!("{path} is not UTF-8: {error}"))
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn supported(&self, running: &str) -> Result<(), String> {
        let held = semver::Version::parse(running.trim_start_matches('v'))
            .map_err(|error| format!("cannot parse running version {running}: {error}"))?;
        let floor = &self.manifest.schema.version;
        let least = semver::Version::parse(floor.trim_start_matches('v'))
            .map_err(|error| format!("cannot parse depot floor {floor}: {error}"))?;
        if !supports(&held, &least) {
            return Err(format!(
                "the installed configuration declares a floor of {floor}, above the running {running}"
            ));
        }
        Ok(())
    }

    pub fn mark(&self) -> &str {
        &self.manifest.metadata.version
    }
}

pub fn supports(running: &semver::Version, floor: &semver::Version) -> bool {
    let mut held = running.clone();
    if floor.pre.is_empty() {
        held.pre = semver::Prerelease::EMPTY;
    }
    held >= *floor
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
        component(&held.metadata.version, "depot version")?;
        let mut paths = BTreeSet::new();
        for object in &held.objects {
            anchored(&object.path)?;
            if !paths.insert(&object.path) {
                return Err(format!("depot manifest repeats {}", object.path));
            }
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

    fn bound(&self, pointer: &Pointer) -> Result<(), String> {
        let metadata = &self.metadata;
        if metadata.version != pointer.version {
            return unbound(pointer);
        }
        if metadata.source != pointer.source {
            return unbound(pointer);
        }
        if metadata.channel != pointer.channel {
            return unbound(pointer);
        }
        if metadata.commit != pointer.commit {
            return unbound(pointer);
        }
        Ok(())
    }
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
        if held.product != "plumb" {
            return Err(format!("depot pointer names product {}", held.product));
        }
        component(&held.version, "depot version")?;
        Ok(held)
    }

    pub fn encode(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|error| format!("cannot encode a depot pointer: {error}"))
    }
}

pub fn root(over: &Path) -> Result<PathBuf, String> {
    if !over.as_os_str().is_empty() {
        return Ok(over.to_path_buf());
    }
    if let Some(path) = crate::config::value("PLUMB_HOME") {
        return Ok(PathBuf::from(path).join("configurations"));
    }
    crate::seat::global("plumb")
        .map(|base| base.join("configurations"))
        .ok_or_else(|| {
            "cannot anchor the plumb configuration seat: no home is declared".to_string()
        })
}

pub fn sha(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn versions(channel: &str, mark: &str) -> String {
    format!("channels/{channel}/versions/{mark}")
}

pub fn latest(channel: &str) -> String {
    format!("channels/{channel}/latest/{POINTER}")
}

fn anchored(path: &str) -> Result<(), String> {
    if path.is_empty() || path.contains('\\') {
        return Err(format!("depot object path is not anchored: {path}"));
    }
    if Path::new(path)
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("depot object path is not anchored: {path}"));
    }
    Ok(())
}

fn component(value: &str, name: &str) -> Result<(), String> {
    anchored(value)?;
    if Path::new(value).components().count() != 1 {
        return Err(format!("{name} is not one path component: {value}"));
    }
    Ok(())
}

fn unbound(pointer: &Pointer) -> Result<(), String> {
    Err(format!(
        "depot pointer {} does not bind its manifest",
        pointer.version
    ))
}
