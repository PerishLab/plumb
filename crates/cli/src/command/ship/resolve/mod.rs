mod plan;
mod workloads;
use super::binding::Binding;
use super::support::{
    Contract, Inventory, carry, contract, embedded, matrix, object, projection, sources, strings,
    text,
};
use crate::command::release;
use plan::{Plan, planned};
use plumb::rig::Rig;
use serde_json::{Value, json};
use std::path::Path;

pub fn run(raw: &str, atom: &str) -> Result<String, String> {
    if atom.len() != 40 || !atom.bytes().all(|held| held.is_ascii_hexdigit()) {
        return Err("--atom must be one full Git commit".into());
    }
    let marker = release::snapshot(raw)?;
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let root = &rig.release.root;
    let spec = marker.spec();
    spec.ship()?;
    let reference = if marker.channel == "stable" {
        format!("refs/heads/release/{}", marker.version)
    } else {
        format!("refs/tags/{}", marker.marker)
    };
    let plan: Value = serde_json::from_str(&release::plan::plan(spec, &reference, &marker.commit)?)
        .map_err(|error| format!("cannot decode release plan: {error}"))?;
    if marker.version != plan["version"] || marker.channel != plan["channel"] {
        return Err("release marker disagrees with its derived ship plan".into());
    }
    let inventory = Inventory::fetch(&rig.workflow.inventory.url)?;
    let world = World {
        marker: &marker.marker,
        binding: Binding::new(spec),
        inventory: inventory.path(),
        source: Some(&rig.workflow.inventory.url),
        root,
    };
    let workload = workloads::run(spec, &marker, &world)?;
    let publication = Publish {
        spec,
        input: &plan["publication"],
        marker: &marker,
        world: &world,
        workload: &workload,
    }
    .run()?;
    serde_json::to_string(&json!({
        "schema": "plumb.ship-graph/v2",
        "product": spec.product,
        "channel": marker.channel,
        "commit": marker.commit,
        "version": marker.version,
        "configuration": spec.configuration,
        "profile": spec.profile,
        "workload": workload.matrix,
        "workload_missing": workload.missing,
        "publication": publication.matrix,
        "publication_missing": publication.missing,
        "publication_ready": !workload.missing,
    }))
    .map_err(|error| format!("cannot encode ship graph: {error}"))
}

struct World<'a> {
    marker: &'a str,
    binding: Binding<'a>,
    inventory: Option<&'a Path>,
    source: Option<&'a str>,
    root: &'a Path,
}

struct Workloads {
    matrix: Value,
    missing: bool,
    reuse: Vec<Value>,
}

struct Publications {
    matrix: Value,
    missing: bool,
}

struct Publish<'a> {
    spec: &'a crate::shape::release::Spec,
    input: &'a Value,
    marker: &'a release::ReleaseMarker,
    world: &'a World<'a>,
    workload: &'a Workloads,
}

impl Publish<'_> {
    fn run(&self) -> Result<Publications, String> {
        let mut pending = Vec::new();
        self.binary(&mut pending)?;
        self.projects(&mut pending)?;
        let missing = !pending.is_empty() || (self.spec.binary() && self.workload.missing);
        Ok(Publications {
            matrix: matrix(pending),
            missing,
        })
    }

    fn binary(&self, pending: &mut Vec<Value>) -> Result<(), String> {
        if !self.spec.binary() || self.workload.missing {
            return Ok(());
        }
        if super::seal::held(self.marker)? {
            return Ok(());
        }
        let projection = projection();
        let roots = sources(self.spec)?;
        let listed = roots.iter().map(String::as_str).collect::<Vec<_>>();
        let node = planned(
            self.world,
            Plan {
                stage: "publication",
                action: "ship/binary",
                projections: &[projection.as_str()],
                roots: &listed,
                runner: "docker",
                workload: Some(self.marker.base()),
                release: Some(&self.marker.version),
                target: Some(&self.marker.commit),
            },
        )?;
        if node["reuse"]["type"] == "url" {
            return Ok(());
        }
        let operation = json!({ "type": "publication", "workloads": self.workload.reuse });
        let request = Binding::new(self.spec).apply(json!({
            "schema": "plumb.ship-request/v2",
            "action": "ship/binary",
            "projections": [projection],
            "roots": roots,
            "operation": operation,
            "reuse": node["reuse"],
            "keys": node["keys"],
        }));
        pending.push(json!({ "runner": "docker", "request": request }));
        Ok(())
    }

    fn projects(&self, pending: &mut Vec<Value>) -> Result<(), String> {
        let rows = self.input["include"]
            .as_array()
            .ok_or("publication plan has no include array")?;
        for entry in rows {
            let action = text(entry, "action")?;
            let binding = contract(action);
            let projections = strings(entry, "projections")?;
            let roots = strings(entry, "roots")?;
            let node = planned(
                self.world,
                Plan {
                    stage: "publication",
                    action,
                    projections: &projections,
                    roots: &roots,
                    runner: "docker",
                    workload: if action == "ship/oci" && self.spec.binary() {
                        Some(self.marker.marker.as_str())
                    } else {
                        embedded(action).then_some(self.marker.base())
                    },
                    release: (binding != Contract::Portable)
                        .then_some(self.marker.version.as_str()),
                    target: (binding == Contract::Exact).then_some(self.marker.commit.as_str()),
                },
            )?;
            if node["reuse"]["type"] == "url" {
                continue;
            }
            let mut request = object(entry)?;
            request.insert("schema".into(), json!("plumb.ship-request/v2"));
            request.insert("reuse".into(), node["reuse"].clone());
            request.insert("keys".into(), node["keys"].clone());
            if action == "ship/oci" {
                carry(&mut request, &self.workload.reuse)?;
            }
            pending.push(json!({ "runner": "docker", "request": request }));
        }
        Ok(())
    }
}
