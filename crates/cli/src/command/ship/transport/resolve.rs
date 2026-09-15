#[path = "../resolve/workloads.rs"]
mod workloads;
use super::binding::Binding;
use super::support::{
    Contract, Inventory, Plan, carry, contract, embedded, matrix, object, projection, sources,
    strings, text,
};
use crate::command::release;
use plumb::rig::Rig;
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn graph(marker: &release::ReleaseMarker, evidence: bool) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = marker.spec();
    if !evidence {
        spec.ship()?;
    }
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
        evidence,
        marker,
        binding: Binding::new(spec),
        inventory: inventory.path(),
        source: Some(&rig.workflow.inventory.url),
        root: &spec.root,
    };
    let workload = workloads::run(spec, marker, &world)?;
    let publication = Publish {
        spec,
        input: &plan["publication"],
        marker,
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

pub(super) struct World<'a> {
    pub evidence: bool,
    pub marker: &'a release::ReleaseMarker,
    pub binding: Binding<'a>,
    pub inventory: Option<&'a Path>,
    pub source: Option<&'a str>,
    pub root: &'a Path,
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
                stage: "",
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
            if action == "ship/oci"
                && super::super::package::publication::image(self.marker)?
                    .resolve(self.world.inventory, self.world.source)?
                    .is_some()
            {
                continue;
            }
            let binding = if action == "ship/oci" && self.spec.binary() {
                Contract::Exact
            } else {
                contract(action)
            };
            let projections = strings(entry, "projections")?;
            let roots = strings(entry, "roots")?;
            let mut node = planned(
                self.world,
                Plan {
                    stage: "",
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
            super::production::reuse(action, &mut node);
            if node["reuse"]["type"] == "url" {
                continue;
            }
            let mut request = object(entry)?;
            request.insert("schema".into(), json!("plumb.ship-request/v2"));
            request.insert("reuse".into(), node["reuse"].clone());
            request.insert("keys".into(), node["keys"].clone());
            if action == "ship/oci" {
                request.insert("production".into(), node["production"].clone());
                request.insert("receipt".into(), node["receipt"].clone());
            }
            if action == "ship/oci" && node["reuse"]["type"] != "workload" {
                carry(&mut request, &self.workload.reuse)?;
            }
            pending.push(json!({ "runner": "docker", "request": request }));
        }
        Ok(())
    }
}

pub(super) fn planned(world: &World<'_>, plan: Plan<'_>) -> Result<Value, String> {
    let mut fields = vec![format!("runner={}", plan.runner)];
    if !plan.stage.is_empty() {
        fields.push(format!("stage={}", plan.stage));
    }
    if let Some(release) = plan.release {
        fields.push(format!("release={release}"));
    }
    if let Some(target) = plan.target {
        fields.push(format!("target={target}"));
    }
    let workload = plan
        .workload
        .map(|value| {
            if plan.stage.is_empty() {
                vec![format!("release={value}")]
            } else {
                vec![format!("release={value}"), format!("stage={}", plan.stage)]
            }
        })
        .unwrap_or_default();
    let named = |values: &[&str]| {
        values
            .iter()
            .map(|value| format!("{}={value}", plan.action))
            .collect()
    };
    let input = crate::command::workflow::plan::Input {
        evidence: world.evidence,
        base: None,
        world: fields,
        workload,
        identity: world.binding.identity(&world.marker.marker),
        project: named(plan.projections),
        roots: named(plan.roots),
        inventory: world.inventory.map(Path::to_path_buf),
        source: world.source.map(str::to_string),
        target: plumb::cli::Root {
            root: world.root.display().to_string(),
        },
    };
    let graph = if plan.stage == "identity/v1" {
        let contract = super::super::native::proof::contract(
            world.marker,
            plan.target.ok_or("binding plan has no target")?,
        )?;
        crate::command::workflow::plan::production(input, plan.action, &contract)?
    } else {
        super::production::plan(input, world.marker, plan.action, plan.target)?
    };
    let graph: Value = serde_json::from_str(&graph)
        .map_err(|error| format!("cannot decode {} plan: {error}", plan.action))?;
    graph["actions"]
        .as_array()
        .and_then(|actions| actions.iter().find(|held| held["name"] == plan.action))
        .cloned()
        .ok_or_else(|| format!("workflow plan omitted {}", plan.action))
}
