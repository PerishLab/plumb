use super::super::adaptor;
use crate::command::release::{artifacts, capsule, output, required, storage};
use plumb::rig::Rig;
use std::process::Command;

use super::request::{Operation, Projection, Request, SCHEMA, Workload};

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
        let version = required("PLUMB_RELEASE_VERSION", &rig.release.version)?;
        let governance = super::binding::Governance::resolve(
            &rig.release.root,
            version,
            self.configuration.is_some() || self.profile.is_some(),
        )?;
        let spec = governance.spec();
        super::binding::Binding::new(spec)
            .verify(self.configuration.as_deref(), self.profile.as_deref())?;
        let release = &rig.release;
        let reuse = self.reuse.encode()?;
        let projection = match self.operation {
            Operation::Bind {
                target,
                archive,
                build,
            } => {
                let workload = super::native::run(super::native::Request {
                    spec,
                    release,
                    action: &self.action,
                    target: &target,
                    archive: &archive,
                    build,
                })?;
                let keys = self
                    .keys
                    .ok_or("identity request carries no inventory keys")?;
                crate::command::workflow::record::project(
                    crate::command::workflow::record::Project {
                        action: &self.action,
                        keys: &keys.to_string(),
                        workload,
                        reuse: None,
                        publication: None,
                        depot: None,
                    },
                )?;
                return result("workload", "", None);
            }
            Operation::Publication { workloads } => {
                let authority = super::support::authority(&rig.publish, &spec.product)?;
                crate::command::release::Product::new(spec).promote(release)?;
                let artifacts = artifacts(release)?;
                materialize(spec, version, &artifacts, &workloads)?;
                super::super::package::product(spec).assemble(version, &artifacts)?;
                crate::command::release::Product::new(spec).compile(release)?;
                let capsule = capsule(release)?;
                storage::publish(&capsule, &authority)?;
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
                &adaptor::registry::registry(spec),
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
                        spec,
                    }
                    .exact(&reuse)?,
                )
            }
            Operation::Chart => {
                Some(adaptor::chart::chart(spec).exact(version, &release.credential, &reuse)?)
            }
            Operation::Npm { package } => {
                pnpm(&spec.root, self.reuse.kind == "none")?;
                Some(adaptor::module::module(spec).exact(
                    &package,
                    version,
                    &release.credential,
                    &reuse,
                )?)
            }
            Operation::Oci { workloads } => {
                let artifacts = artifacts(release)?;
                materialize(spec, version, &artifacts, &workloads)?;
                Some(adaptor::container::run(
                    &adaptor::image::image(spec),
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

fn materialize(
    spec: &crate::shape::release::Spec,
    version: &str,
    root: &std::path::Path,
    workloads: &[Workload],
) -> Result<(), String> {
    std::fs::create_dir_all(root)
        .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
    for workload in workloads {
        if workload.target.is_empty() || workload.archive.is_empty() || workload.url.is_empty() {
            return Err("binary publication carries an incomplete workload".into());
        }
        let target = root.join(&workload.archive);
        if spec.target(&workload.target)?.archive != workload.archive || target.exists() {
            return Err("binary workload has an invalid or occupied archive path".into());
        }
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
        super::native::verify(
            spec,
            &crate::command::release::snapshot(version)?,
            &target,
            &workload.target,
        )?;
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
