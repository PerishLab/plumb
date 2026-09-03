use super::super::adaptor;
use crate::command::release::{artifacts, capsule, output, required, storage};
use plumb::rig::Rig;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

const SCHEMA: &str = "plumb.ship-request/v2";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    action: String,
    projections: Vec<String>,
    roots: Vec<String>,
    operation: Operation,
    #[serde(default)]
    profile: Option<String>,
    #[serde(default = "Reuse::none")]
    reuse: Reuse,
    #[serde(default)]
    keys: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
enum Operation {
    Workload {
        target: String,
        archive: String,
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
struct Workload {
    target: String,
    archive: String,
    url: String,
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
    #[serde(default)]
    depot: Option<serde_json::Value>,
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
        let spec = crate::shape::release::Spec::resolve(&rig.release.root)?;
        if self.profile != spec.profile {
            return Err("ship request product profile differs from the active depot".into());
        }
        let release = &rig.release;
        let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
        let reuse = self.reuse.encode()?;
        let projection = match self.operation {
            Operation::Workload { target, archive } => {
                super::super::package::product(&spec).build(super::super::package::Build {
                    target: &target,
                    version,
                    channel: required("PLUMB_RELEASE_CHANNEL", &release.channel)?,
                    commit: required("PLUMB_RELEASE_COMMIT", &release.commit)?,
                    artifacts: &artifacts(release)?,
                })?;
                let keys = self
                    .keys
                    .ok_or_else(|| "an exact ship request carries no inventory keys".to_string())?;
                crate::command::workflow::record::project(
                    crate::command::workflow::record::Project {
                        action: &self.action,
                        keys: &keys.to_string(),
                        workload: artifacts(release)?.join(archive),
                        reuse: None,
                        publication: None,
                        depot: None,
                    },
                )?;
                return result("workload", "", None);
            }
            Operation::Publication { workloads } => {
                super::support::authority(&rig.publish, &spec.product)?;
                crate::command::release::Product::new(&spec).promote(release)?;
                let artifacts = artifacts(release)?;
                materialize(&artifacts, &workloads)?;
                super::super::package::product(&spec).assemble(version, &artifacts)?;
                crate::command::release::Product::new(&spec).compile(release)?;
                let capsule = capsule(release)?;
                storage::publish(&capsule, &rig.publish)?;
                let (compiled, _) = crate::command::release::record::Capsule::read(&capsule)?;
                let publication = compiled.seal.remote.url;
                let keys = self
                    .keys
                    .ok_or_else(|| "an exact ship request carries no inventory keys".to_string())?;
                crate::command::workflow::record::project(
                    crate::command::workflow::record::Project {
                        action: &self.action,
                        keys: &keys.to_string(),
                        workload: output(release)?.join("capsule.json"),
                        reuse: None,
                        publication: Some(publication.clone()),
                        depot: None,
                    },
                )?;
                return result("url", &publication, None);
            }
            Operation::Cargo => Some(super::super::package::project::cargo(
                &adaptor::registry::registry(&spec),
                version,
                &release.credential,
                &reuse,
            )?),
            Operation::Cfworker => {
                pnpm(&spec.root, self.reuse.kind == "none")?;
                Some(
                    super::super::site::Worker {
                        root: &spec.root,
                        version,
                    }
                    .exact(&reuse)?,
                )
            }
            Operation::Chart => {
                Some(adaptor::chart::chart(&spec).exact(version, &release.credential, &reuse)?)
            }
            Operation::Npm { package } => {
                pnpm(&spec.root, self.reuse.kind == "none")?;
                Some(adaptor::module::module(&spec).exact(
                    &package,
                    version,
                    &release.credential,
                    &reuse,
                )?)
            }
            Operation::Oci { workloads } => {
                let artifacts = artifacts(release)?;
                materialize(&artifacts, &workloads)?;
                Some(adaptor::container::run(
                    &adaptor::image::image(&spec),
                    adaptor::container::Request {
                        version,
                        commit: &release.commit,
                        artifacts: &artifacts,
                        credential: &release.credential,
                        reuse: &reuse,
                    },
                )?)
            }
        };
        let Some(projection) = projection else {
            return result("none", "", None);
        };
        let projection: Projection = serde_json::from_str(&projection)
            .map_err(|error| format!("cannot read adaptor result: {error}"))?;
        let keys = self
            .keys
            .ok_or_else(|| "an exact ship request carries no inventory keys".to_string())?;
        crate::command::workflow::record::project(crate::command::workflow::record::Project {
            action: &self.action,
            keys: &keys.to_string(),
            workload: projection.workload,
            reuse: (self.reuse.kind == "workload").then_some(self.reuse.source.as_str()),
            publication: Some(projection.publication.clone()),
            depot: projection.depot.clone(),
        })?;
        result("url", &projection.publication, projection.depot)
    }
}

fn materialize(root: &std::path::Path, workloads: &[Workload]) -> Result<(), String> {
    std::fs::create_dir_all(root)
        .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
    for workload in workloads {
        if workload.target.is_empty() || workload.archive.is_empty() || workload.url.is_empty() {
            return Err("binary publication carries an incomplete workload".into());
        }
        let target = root.join(&workload.archive);
        let status = Command::new("curl")
            .args([
                "--fail",
                "--silent",
                "--show-error",
                "--location",
                "--retry",
                "3",
                "--output",
            ])
            .arg(&target)
            .arg(&workload.url)
            .status()
            .map_err(|error| format!("cannot fetch {}: {error}", workload.url))?;
        if !status.success() {
            return Err(format!(
                "cannot fetch binary workload for {}",
                workload.target
            ));
        }
    }
    Ok(())
}

fn pnpm(root: &std::path::Path, install: bool) -> Result<(), String> {
    if !root.join("pnpm-lock.yaml").is_file() {
        return Ok(());
    }
    command(root, "corepack", &["enable"])?;
    if !install {
        return Ok(());
    }
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

fn result(kind: &str, source: &str, depot: Option<serde_json::Value>) -> Result<String, String> {
    serde_json::to_string(&serde_json::json!({
        "schema": "plumb.ship-result/v1",
        "result": { "type": kind, "source": source },
        "depot": depot,
    }))
    .map_err(|error| format!("cannot encode ship result: {error}"))
}
