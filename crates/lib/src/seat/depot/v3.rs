use serde::{Deserialize, Serialize};

mod kind;
mod local;
mod manifest;
mod pointer;
mod route;
mod source;
mod value;

pub use kind::Kind;
pub use local::{generation as local_generation, install};
pub use manifest::Manifest;
pub use pointer::Pointer;
pub use route::{Route, generation, latest, manifest};
pub use source::Bundle;

pub const FORMAT: u32 = 3;
pub const LEAF: &str = "manifest.json";
pub const POINTER: &str = "latest.json";

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
