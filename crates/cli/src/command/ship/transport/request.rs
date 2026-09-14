use serde::Deserialize;
use std::path::PathBuf;

pub(super) const SCHEMA: &str = "plumb.ship-request/v2";
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    pub(super) schema: String,
    pub(super) action: String,
    pub(super) projections: Vec<String>,
    pub(super) roots: Vec<String>,
    pub(super) operation: Operation,
    #[serde(default)]
    pub(super) configuration: Option<String>,
    #[serde(default)]
    pub(super) profile: Option<String>,
    #[serde(default = "Reuse::none")]
    pub(super) reuse: Reuse,
    #[serde(default)]
    pub(super) keys: Option<serde_json::Value>,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub(super) enum Operation {
    Bind {
        target: String,
        archive: String,
        build: super::native::Build,
    },
    Publication {
        workloads: Vec<Workload>,
    },
    Cargo,
    Cfworker,
    Chart,
    Npm {
        package: String,
    },
    Oci {
        #[serde(default)]
        workloads: Vec<Workload>,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Workload {
    pub(super) target: String,
    pub(super) archive: String,
    pub(super) url: String,
}
#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reuse {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) source: String,
}
#[derive(Deserialize)]
pub(super) struct Projection {
    pub(super) workload: PathBuf,
    pub(super) publication: String,
    #[serde(default)]
    pub(super) depot: Option<serde_json::Value>,
}

impl Reuse {
    pub(super) fn none() -> Self {
        Self {
            kind: "none".into(),
            source: String::new(),
        }
    }

    pub(super) fn encode(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|error| format!("cannot encode reuse carrier: {error}"))
    }
}
