use super::super::adaptor;
use super::binding::materialize;
use super::model::{Operation, Projection, Request};
use crate::command::release::{artifacts, capsule, storage};
use plumb::rig::Rig;

pub fn run(text: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| format!("cannot parse --request: {error}"))?;
    let context = (value["schema"] == "plumb.blob-execution/v1").then_some(value);
    let request: Request = if let Some(context) = &context {
        serde_json::from_value(context["payload"].clone())
    } else {
        serde_json::from_str(text)
    }
    .map_err(|error| format!("cannot parse --request: {error}"))?;
    if request.schema != "plumb.ship-request/v3" {
        return Err("ship request requires plumb.ship-request/v3".into());
    }
    request.run(context.as_ref())
}

impl Request {
    fn run(mut self, context: Option<&serde_json::Value>) -> Result<String, String> {
        if context.is_none()
            && matches!(
                self.operation,
                Operation::Produce { .. } | Operation::Package { .. }
            ) != self.input.is_some()
        {
            return Err("only production requires an explicit input snapshot".into());
        }
        let mut rig = Rig::resolve(None).map_err(|error| error.to_string())?;
        let governance = super::binding::Governance::resolve(&self.marker)?;
        governance.apply(&mut rig.release)?;
        let marker = governance.marker();
        let spec = governance.spec();
        super::binding::Binding::new(spec)
            .verify(Some(&self.configuration), Some(&self.profile))?;
        governance.request(&serde_json::json!({
            "action": self.action, "operation": self.operation,
        }))?;
        if let Some(context) = context {
            self = serde_json::from_value(context::request(context)?)
                .map_err(|error| error.to_string())?;
        }
        super::super::package::project::Source::parse(&self.reuse.encode()?)?;
        let contract = self.contract(marker)?;
        if self.production
            != contract
                .as_ref()
                .map(plumb::rule::Production::digest)
                .transpose()?
        {
            return Err(
                "ship production contract differs from its marker and implementation".into(),
            );
        }
        let projection = self.perform(&rig, marker, contract.as_ref())?;
        let evidence = match &self.operation {
            Operation::Produce { .. } | Operation::Bind { .. } => {
                serde_json::to_value(&projection.receipt).map_err(|error| error.to_string())?
            }
            _ => serde_json::json!({
                "schema": "plumb.ship-resource/v1", "marker": marker.digest()?,
                "action": self.action, "source": projection.publication,
                "content": crate::command::release::record::digest(&projection.workload)?.0,
                "depot": projection.depot,
                "production": projection.receipt,
            }),
        };
        serde_json::to_string(&serde_json::json!({
            "schema": "plumb.ship-result/v2", "marker": marker.digest()?,
            "action": self.action, "projection": projection, "evidence": evidence,
        }))
        .map_err(|error| error.to_string())
    }

    fn contract(
        &self,
        marker: &crate::command::release::ReleaseMarker,
    ) -> Result<Option<plumb::rule::Production>, String> {
        match &self.operation {
            Operation::Produce { target, .. } => {
                super::production::contract(marker, target).map(Some)
            }
            Operation::Bind { target, .. } => {
                super::super::native::proof::contract(marker, target).map(Some)
            }
            Operation::Oci { .. } => super::super::package::production::contract(marker).map(Some),
            _ => Ok(None),
        }
    }

    fn perform(
        &self,
        rig: &Rig,
        marker: &crate::command::release::ReleaseMarker,
        contract: Option<&plumb::rule::Production>,
    ) -> Result<Projection, String> {
        let spec = marker.spec();
        let release = &rig.release;
        let version = release.version.as_str();
        let reuse = self.reuse.encode()?;
        if matches!(
            self.operation,
            Operation::Cargo | Operation::Chart | Operation::Npm { .. } | Operation::Cfworker
        ) && self.reuse.kind != "workload"
        {
            return Err("package publication requires its declared production workload".into());
        }
        let credential = if matches!(
            self.operation,
            Operation::Cargo | Operation::Chart | Operation::Npm { .. } | Operation::Oci { .. }
        ) {
            crate::command::release::authority::credential(&release.credential)?
        } else {
            String::new()
        };
        let projection = match &self.operation {
            Operation::Package { operation } => {
                return super::media::produce(
                    marker,
                    operation,
                    self.input
                        .as_deref()
                        .ok_or("package production needs materialized source")?,
                );
            }
            Operation::Complete { resources } => {
                let body = super::receipt::publish(marker, &rig.publish, resources)?;
                let output = artifacts(release)?.join("ship.json");
                std::fs::create_dir_all(output.parent().ok_or("completion output has no parent")?)
                    .map_err(|error| error.to_string())?;
                std::fs::write(&output, body).map_err(|error| error.to_string())?;
                return Ok(Projection {
                    workload: output,
                    publication: format!("{}/{}", marker.authority, super::receipt::key(marker)),
                    receipt: None,
                    depot: None,
                });
            }
            Operation::Produce { target, archive } => {
                let output = artifacts(release)?;
                let input = self
                    .input
                    .as_deref()
                    .ok_or("production requires a verified input snapshot")?;
                let receipt = super::production::produce(marker, target, &output, input)?;
                return Ok(Projection {
                    workload: output.join(archive),
                    publication: String::new(),
                    receipt: Some(receipt),
                    depot: None,
                });
            }
            Operation::Bind {
                target,
                archive,
                build,
            } => {
                let (workload, receipt) =
                    super::super::native::run(super::super::native::Request {
                        spec,
                        release,
                        target,
                        archive,
                        build: build.as_ref(),
                    })?;
                return Ok(Projection {
                    workload,
                    publication: String::new(),
                    receipt: Some(receipt),
                    depot: None,
                });
            }
            Operation::Publication { workloads } => {
                let authority = super::support::authority(&rig.publish, &spec.product)?;
                crate::command::release::Product::new(spec).promote(release)?;
                let artifacts = artifacts(release)?;
                materialize(&artifacts, workloads, marker, false)?;
                super::super::package::product(spec).assemble(version, &artifacts)?;
                crate::command::release::Product::new(spec).compile(release)?;
                let path = capsule(release)?;
                storage::publish(&path, &authority)?;
                let (compiled, _) = crate::command::release::record::Capsule::read(&path)?;
                return Ok(Projection {
                    workload: path,
                    publication: compiled.seal.remote.url,
                    receipt: None,
                    depot: None,
                });
            }
            Operation::Cargo => super::super::package::project::cargo(
                &adaptor::registry::registry(spec),
                version,
                &credential,
                &reuse,
            )?,
            Operation::Cfworker => super::super::site::Worker {
                root: &spec.root,
                version,
                spec,
            }
            .exact(&reuse)?,
            Operation::Chart => adaptor::chart::chart(spec).exact(version, &credential, &reuse)?,
            Operation::Npm { package } => {
                adaptor::module::module(spec).exact(package, version, &credential, &reuse)?
            }
            Operation::Oci { workloads } => {
                let artifacts = artifacts(release)?;
                if self.reuse.kind == "none" {
                    materialize(&artifacts, workloads, marker, true)?;
                }
                adaptor::container::run(
                    &adaptor::image::image(spec),
                    adaptor::container::Request {
                        version,
                        commit: &release.commit,
                        artifacts: &artifacts,
                        credential: &credential,
                        reuse: &reuse,
                        proof: super::super::package::production::Proof {
                            contract: contract.ok_or("image request has no production contract")?,
                            receipt: self.receipt.as_ref(),
                        },
                    },
                )?
            }
        };
        let projection: Projection = serde_json::from_str(&projection)
            .map_err(|error| format!("cannot read adaptor result: {error}"))?;
        if matches!(self.operation, Operation::Oci { .. }) {
            super::super::package::publication::image(marker)?.verify(&projection.publication)?;
        }
        Ok(projection)
    }
}
#[path = "context.rs"]
mod context;
