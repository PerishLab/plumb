use super::super::adaptor;
use super::binding::{Workload, materialize};
use super::support::pnpm;
use crate::command::release::{artifacts, capsule, output, required, storage};
use plumb::rig::Rig;
use serde::Deserialize;
use std::path::PathBuf;

const SCHEMA: &str = "plumb.ship-request/v2";
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    action: String,
    projections: Vec<String>,
    roots: Vec<String>,
    operation: Operation,
    configuration: String,
    profile: String,
    #[serde(default = "Reuse::none")]
    reuse: Reuse,
    keys: serde_json::Value,
    #[serde(default)]
    production: Option<String>,
    #[serde(default)]
    receipt: Option<plumb::rule::Receipt>,
}
#[derive(Deserialize, serde::Serialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
enum Operation {
    Workload {
        target: String,
        archive: String,
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
    receipt: Option<plumb::rule::Receipt>,
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
        if self.configuration.trim().is_empty()
            || self.profile.trim().is_empty()
            || !self.keys.is_object()
        {
            return Err(
                "ship request requires marker-bound configuration, profile and inventory keys"
                    .into(),
            );
        }
        let keys = crate::command::workflow::record::keys(&self.keys.to_string())
            .map_err(|error| format!("ship request inventory keys: {error}"))?;
        if !matches!(self.operation, Operation::Workload { .. }) && keys.publication.is_none() {
            return Err("ship publication request carries no publication key".into());
        }
        super::super::package::project::Source::parse(&self.reuse.encode()?)?;
        let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
        let governance = super::binding::Governance::resolve(required(
            "PLUMB_RELEASE_VERSION",
            &rig.release.version,
        )?)?;
        governance.apply(&mut rig.release)?;
        let version = rig.release.version.as_str();
        let spec = governance.spec();
        super::binding::Binding::new(spec)
            .verify(Some(&self.configuration), Some(&self.profile))?;
        governance.request(&serde_json::json!({
            "action":self.action, "projections":self.projections,
            "roots":self.roots, "operation":self.operation,
            "keys":self.keys, "production":self.production,
        }))?;
        let release = &rig.release;
        let binding = if matches!(self.operation, Operation::Oci { .. }) {
            let binding = super::super::package::publication::image(governance.marker())?;
            if let Some(source) = binding.resolve(None, Some(&rig.workflow.inventory.url))? {
                return result("url", &source, None);
            }
            Some(binding)
        } else {
            None
        };
        let reuse = self.reuse.encode()?;
        let production = binding
            .as_ref()
            .map(|_| super::super::package::production::contract(governance.marker()))
            .transpose()?;
        if let Some(contract) = &production
            && self.production.as_deref() != Some(contract.digest()?.as_str())
        {
            return Err("image request production contract differs from its dispatch configuration and implementation".into());
        }
        let projection = match self.operation {
            Operation::Workload { target, archive } => {
                super::production::execute(
                    governance.marker(),
                    super::production::Input {
                        target: &target,
                        archive: &archive,
                        action: &self.action,
                        keys: Some(&self.keys),
                        contract: self.production.as_deref(),
                    },
                )?;
                return result("workload", "", None);
            }
            Operation::Publication { workloads } => {
                let authority = super::support::authority(&rig.publish, &spec.product)?;
                crate::command::release::Product::new(spec).promote(release)?;
                let artifacts = artifacts(release)?;
                materialize(&artifacts, &workloads, governance.marker(), false)?;
                super::super::package::product(spec).assemble(version, &artifacts)?;
                crate::command::release::Product::new(spec).compile(release)?;
                let capsule = capsule(release)?;
                storage::publish(&capsule, &authority)?;
                let (compiled, _) = crate::command::release::record::Capsule::read(&capsule)?;
                let publication = compiled.seal.remote.url;
                let keys = self.keys;
                crate::command::workflow::record::project(
                    crate::command::workflow::record::Project {
                        action: &self.action,
                        keys: &keys.to_string(),
                        workload: output(release)?.join("capsule.json"),
                        reuse: None,
                        publication: Some(publication.clone()),
                        depot: None,
                        production: None,
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
                if self.reuse.kind == "none" {
                    materialize(&artifacts, &workloads, governance.marker(), true)?;
                }
                Some(adaptor::container::run(
                    &adaptor::image::image(spec),
                    adaptor::container::Request {
                        version,
                        commit: &release.commit,
                        artifacts: &artifacts,
                        credential: &release.credential,
                        reuse: &reuse,
                        proof: super::super::package::production::Proof {
                            contract: production
                                .as_ref()
                                .ok_or("image request has no production contract")?,
                            receipt: self.receipt.as_ref(),
                        },
                    },
                )?)
            }
        };
        let Some(projection) = projection else {
            return result("none", "", None);
        };
        let projection: Projection = serde_json::from_str(&projection)
            .map_err(|error| format!("cannot read adaptor result: {error}"))?;
        if let Some(binding) = &binding {
            binding.verify(&projection.publication)?;
        }
        let keys = self.keys;
        crate::command::workflow::record::bound(
            crate::command::workflow::record::Project {
                action: &self.action,
                keys: &keys.to_string(),
                workload: projection.workload,
                reuse: (self.reuse.kind == "workload").then_some(self.reuse.source.as_str()),
                publication: Some(projection.publication.clone()),
                depot: projection.depot.clone(),
                production: production
                    .as_ref()
                    .map(|contract| {
                        projection
                            .receipt
                            .map(|receipt| (contract, receipt))
                            .ok_or_else(|| {
                                "image projection carries no production receipt".to_string()
                            })
                    })
                    .transpose()?,
            },
            binding.as_ref().map(|binding| binding.binding.key.as_str()),
        )?;
        result("url", &projection.publication, projection.depot)
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
