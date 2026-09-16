use super::binding::Workload;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    pub(super) schema: String,
    pub(super) marker: String,
    pub(super) action: String,
    pub(super) operation: Operation,
    pub(super) configuration: String,
    pub(super) profile: String,
    #[serde(default)]
    pub(super) input: Option<PathBuf>,
    #[serde(default = "Reuse::none")]
    pub(super) reuse: Reuse,
    #[serde(default)]
    pub(super) production: Option<String>,
    #[serde(default)]
    pub(super) receipt: Option<plumb::rule::Receipt>,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub(super) enum Operation {
    Package {
        operation: Box<Operation>,
    },
    Complete {
        #[serde(skip_serializing)]
        resources: std::collections::BTreeMap<String, serde_json::Value>,
    },
    Produce {
        target: String,
        archive: String,
    },
    Bind {
        target: String,
        archive: String,
        build: Box<super::super::native::Build>,
    },
    Publication {
        #[serde(skip_serializing)]
        workloads: Vec<Workload>,
    },
    Cargo,
    Cfworker,
    Chart,
    Npm {
        package: String,
    },
    Oci {
        #[serde(default, skip_serializing)]
        workloads: Vec<Workload>,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reuse {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) source: String,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Projection {
    pub(super) workload: PathBuf,
    pub(super) publication: String,
    #[serde(default)]
    pub(super) receipt: Option<plumb::rule::Receipt>,
    #[serde(default)]
    pub(super) depot: Option<serde_json::Value>,
}

impl Reuse {
    fn none() -> Self {
        Self {
            kind: "none".into(),
            source: String::new(),
        }
    }

    pub(super) fn encode(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|error| error.to_string())
    }
}
