use serde::{Deserialize, Serialize};

mod cache;
#[cfg(feature = "depot")]
mod discovery;
mod local;
mod manifest;
mod pinned;
mod pointer;
#[cfg(feature = "depot")]
mod remote;
mod route;
mod source;
mod value;

pub use cache::Cache;
#[cfg(feature = "depot")]
pub use discovery::{Channel, Discovered};
pub use local::install;
pub use manifest::Manifest;
pub use pinned::Pinned;
pub use pointer::Pointer;
#[cfg(feature = "depot")]
pub use remote::{Exact, Generation, Query};
pub use route::{Route, channel, generation, latest, manifest};
pub use source::Bundle;
#[cfg(any(test, debug_assertions, feature = "testing"))]
pub(crate) use source::media;

pub const FORMAT: u32 = 3;
pub const LEAF: &str = "manifest.json";
pub const POINTER: &str = "latest.json";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Configuration,
    Changelog,
    Skill,
    Scene,
    Worker,
    Manager,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::Changelog => "changelog",
            Self::Skill => "skill",
            Self::Scene => "scene",
            Self::Worker => "worker",
            Self::Manager => "manager",
        }
    }

    pub fn directory(self) -> &'static str {
        match self {
            Self::Configuration => "configurations",
            Self::Changelog => "changelogs",
            Self::Skill => "skills",
            Self::Scene => "scenes",
            Self::Worker => "workers",
            Self::Manager => "managers",
        }
    }
}

pub fn check(source: &str) -> Result<(), String> {
    value::Value(source).source()
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub name: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub product: String,
    pub channel: String,
    pub version: String,
    pub marker: Marker,
    pub kind: Kind,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Object {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    #[serde(rename = "mediaType")]
    pub media: String,
    pub executable: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

pub struct Publication<'a> {
    pub source: &'a str,
    pub prior: Option<String>,
    pub created: String,
}
