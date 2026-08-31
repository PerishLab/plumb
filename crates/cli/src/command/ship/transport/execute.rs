use super::super::adaptor;
use crate::command::release::{artifacts, required};
use plumb::rig::Rig;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

const SCHEMA: &str = "plumb.ship-request/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    action: String,
    projections: Vec<String>,
    roots: Vec<String>,
    operation: Operation,
    #[serde(default = "Reuse::none")]
    reuse: Reuse,
    #[serde(default)]
    keys: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
enum Operation {
    Cargo,
    Cfworker,
    Chart,
    Npm { package: String },
    Oci,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Reuse {
    #[serde(rename = "type")]
    kind: String,
    source: String,
}

#[derive(Deserialize)]
struct Projection {
    workload: PathBuf,
    publication: String,
}

impl Reuse {
    fn none() -> Self {
        Self {
            kind: "none".into(),
            source: String::new(),
        }
    }

    fn encode(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|error| format!("cannot encode reuse carrier: {error}"))
    }
}

pub fn run(text: &str) -> Result<String, String> {
    let request: Request =
        serde_json::from_str(text).map_err(|error| format!("cannot parse --request: {error}"))?;
    request.run()
}

impl Request {
    fn run(self) -> Result<String, String> {
        if self.schema != SCHEMA {
            return Err(format!("ship request schema must be {SCHEMA}"));
        }
        if self.action.trim().is_empty() || self.projections.iter().any(|held| held.is_empty()) {
            return Err("ship request carries an empty planning identity".into());
        }
        if self.roots.is_empty() || self.roots.iter().any(|held| held.is_empty()) {
            return Err("ship request must carry at least one non-empty root".into());
        }
        let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
        let spec = crate::shape::release::Spec::read(&rig.release.root.join("plumb.toml"))?;
        let release = &rig.release;
        let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
        let reuse = self.reuse.encode()?;
        if matches!(
            &self.operation,
            Operation::Cargo | Operation::Chart | Operation::Npm { .. } | Operation::Oci
        ) {
            super::super::attachment::sealed(&spec, release, version, true)?;
        }
        let projection = match self.operation {
            Operation::Cargo => Some(super::super::package::project::cargo(
                &adaptor::registry::registry(&spec),
                version,
                &release.credential,
                &reuse,
            )?),
            Operation::Cfworker => {
                if self.reuse.kind == "none" {
                    install(&spec.root)?;
                }
                Some(
                    super::super::site::Worker {
                        root: &spec.root,
                        channel: required("PLUMB_RELEASE_CHANNEL", &release.channel)?,
                        version,
                    }
                    .exact(&reuse)?,
                )
            }
            Operation::Chart => {
                Some(adaptor::chart::chart(&spec).exact(version, &release.credential, &reuse)?)
            }
            Operation::Npm { package } => {
                if self.reuse.kind == "none" {
                    install(&spec.root)?;
                }
                Some(adaptor::module::module(&spec).exact(
                    &package,
                    version,
                    &release.credential,
                    &reuse,
                )?)
            }
            Operation::Oci => Some(adaptor::container::run(
                &adaptor::image::image(&spec),
                adaptor::container::Request {
                    version,
                    commit: &release.commit,
                    artifacts: &artifacts(release)?,
                    credential: &release.credential,
                    reuse: &reuse,
                },
            )?),
        };
        let Some(projection) = projection else {
            return result("none", "");
        };
        let projection: Projection = serde_json::from_str(&projection)
            .map_err(|error| format!("cannot read adaptor result: {error}"))?;
        let keys = self
            .keys
            .ok_or_else(|| "an exact ship request carries no inventory keys".to_string())?;
        crate::command::workflow::record::project(
            &self.action,
            &keys.to_string(),
            projection.workload,
            Some(projection.publication.clone()),
        )?;
        result("url", &projection.publication)
    }
}

fn install(root: &std::path::Path) -> Result<(), String> {
    if !root.join("pnpm-lock.yaml").is_file() {
        return Ok(());
    }
    command(root, "corepack", &["enable"])?;
    let store = root.join("target/pnpm-store");
    command(
        root,
        "pnpm",
        &[
            "install",
            "--frozen-lockfile",
            "--store-dir",
            &store.to_string_lossy(),
        ],
    )
}

fn command(root: &std::path::Path, program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|error| format!("cannot run {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{program} failed while materializing a ship request"
        ))
    }
}

fn result(kind: &str, source: &str) -> Result<String, String> {
    serde_json::to_string(&serde_json::json!({
        "schema": "plumb.ship-result/v1",
        "result": { "type": kind, "source": source },
    }))
    .map_err(|error| format!("cannot encode ship result: {error}"))
}
