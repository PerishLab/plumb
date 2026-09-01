use crate::command::release;
use plumb::rig::Rig;
use serde_json::{Value, json};
use std::path::Path;

use super::support::{Inventory, object, projection, sources, strings, text};

const SCHEMA: &str = "plumb.ship-graph/v2";

pub fn run(raw: &str, atom: &str) -> Result<String, String> {
    if atom.len() != 40 || !atom.bytes().all(|held| held.is_ascii_hexdigit()) {
        return Err("--atom must be one full Git commit".into());
    }
    let marker = release::snapshot(raw)?;
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let root = &rig.release.root;
    let spec = crate::shape::release::Spec::read(&root.join("plumb.toml"))?;
    let reference = if marker.channel == "stable" {
        format!("refs/heads/release/{}", marker.version)
    } else {
        format!("refs/tags/{}", marker.marker)
    };
    let plan: Value =
        serde_json::from_str(&release::plan::plan(&spec, &reference, &marker.commit)?)
            .map_err(|error| format!("cannot decode release plan: {error}"))?;
    if marker.version != plan["version"] || marker.channel != plan["channel"] {
        return Err("release marker disagrees with its derived ship plan".into());
    }
    let inventory = Inventory::fetch(&rig.workflow.inventory.url)?;
    let world = World {
        atom,
        plumb: env!("CARGO_PKG_VERSION"),
        marker: &marker.marker,
        inventory: inventory.path(),
        source: Some(&rig.workflow.inventory.url),
        root,
    };
    let workload = workloads(&spec, &marker, &world)?;
    let publication = Publish {
        spec: &spec,
        input: &plan["publication"],
        marker: &marker,
        world: &world,
        workload: &workload,
    }
    .run()?;
    serde_json::to_string(&json!({
        "schema": SCHEMA,
        "product": spec.product,
        "channel": marker.channel,
        "commit": marker.commit,
        "version": marker.version,
        "workload": workload.matrix,
        "workload_missing": workload.missing,
        "publication": publication.matrix,
        "publication_missing": publication.missing,
        "publication_ready": !workload.missing,
    }))
    .map_err(|error| format!("cannot encode ship graph: {error}"))
}

struct World<'a> {
    atom: &'a str,
    plumb: &'a str,
    marker: &'a str,
    inventory: Option<&'a Path>,
    source: Option<&'a str>,
    root: &'a Path,
}

struct Workloads {
    matrix: Value,
    missing: bool,
    reuse: Vec<Value>,
}

fn workloads(
    spec: &crate::shape::release::Spec,
    marker: &release::ReleaseMarker,
    world: &World<'_>,
) -> Result<Workloads, String> {
    if marker.channel == "stable" || !spec.binary() {
        return Ok(Workloads {
            matrix: idle(),
            missing: false,
            reuse: Vec::new(),
        });
    }
    let targets: Value = serde_json::from_str(&super::super::package::product(spec).matrix()?)
        .map_err(|error| format!("cannot decode binary matrix: {error}"))?;
    let projection = projection(world.root);
    let roots = sources(spec)?;
    let listed = roots.iter().map(String::as_str).collect::<Vec<_>>();
    let mut pending = Vec::new();
    let mut reuse = Vec::new();
    for target in targets["include"]
        .as_array()
        .ok_or("binary matrix has no include array")?
    {
        let triple = text(target, "target")?;
        let runner = text(target, "runner")?;
        let archive = text(target, "archive")?;
        let action = format!("ship/binary.{triple}");
        let node = planned(
            world,
            Plan {
                action: &action,
                projections: &[projection.as_str()],
                roots: &listed,
                runner,
                release: Some(&marker.version),
                target: Some(triple),
            },
        )?;
        if node["reuse"]["type"] == "workload" {
            reuse.push(json!({
                "target": triple,
                "archive": archive,
                "url": node["reuse"]["source"],
            }));
            continue;
        }
        let request = json!({
            "schema": "plumb.ship-request/v2",
            "action": action,
            "projections": [projection],
            "roots": roots,
            "operation": {
                "type": "workload",
                "target": triple,
                "archive": archive,
            },
            "reuse": node["reuse"],
            "keys": node["keys"],
        });
        pending.push(json!({ "runner": runner, "request": request }));
    }
    let missing = !pending.is_empty();
    Ok(Workloads {
        matrix: matrix(pending),
        missing,
        reuse,
    })
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
        let missing = !pending.is_empty();
        Ok(Publications {
            matrix: matrix(pending),
            missing,
        })
    }

    fn binary(&self, pending: &mut Vec<Value>) -> Result<(), String> {
        if !self.spec.binary() || self.workload.missing {
            return Ok(());
        }
        let projection = projection(self.world.root);
        let roots = sources(self.spec)?;
        let listed = roots.iter().map(String::as_str).collect::<Vec<_>>();
        let node = planned(
            self.world,
            Plan {
                action: "ship/binary",
                projections: &[projection.as_str()],
                roots: &listed,
                runner: "docker",
                release: Some(&self.marker.version),
                target: Some(&self.marker.commit),
            },
        )?;
        if node["reuse"]["type"] == "url" {
            return Ok(());
        }
        let operation = json!({ "type": "publication", "workloads": self.workload.reuse });
        let request = json!({
            "schema": "plumb.ship-request/v2",
            "action": "ship/binary",
            "projections": [projection],
            "roots": roots,
            "operation": operation,
            "reuse": node["reuse"],
            "keys": node["keys"],
        });
        pending.push(json!({ "runner": "docker", "request": request }));
        Ok(())
    }

    fn projects(&self, pending: &mut Vec<Value>) -> Result<(), String> {
        let rows = self.input["include"]
            .as_array()
            .ok_or("publication plan has no include array")?;
        for entry in rows {
            let action = text(entry, "action")?;
            let versioned = action == "ship/cfworker";
            let projections = strings(entry, "projections")?;
            let roots = strings(entry, "roots")?;
            let node = planned(
                self.world,
                Plan {
                    action,
                    projections: &projections,
                    roots: &roots,
                    runner: "docker",
                    release: versioned.then_some(self.marker.version.as_str()),
                    target: versioned.then_some(self.marker.commit.as_str()),
                },
            )?;
            if node["reuse"]["type"] == "url" {
                continue;
            }
            let mut request = object(entry)?;
            request.insert("schema".into(), json!("plumb.ship-request/v2"));
            request.insert("reuse".into(), node["reuse"].clone());
            request.insert("keys".into(), node["keys"].clone());
            pending.push(json!({ "runner": "docker", "request": request }));
        }
        Ok(())
    }
}

fn matrix(include: Vec<Value>) -> Value {
    if include.is_empty() {
        idle()
    } else {
        json!({ "include": include })
    }
}

fn idle() -> Value {
    json!({ "include": [{ "runner": "docker", "control": "reuse" }] })
}

struct Plan<'a> {
    action: &'a str,
    projections: &'a [&'a str],
    roots: &'a [&'a str],
    runner: &'a str,
    release: Option<&'a str>,
    target: Option<&'a str>,
}

fn planned(world: &World<'_>, plan: Plan<'_>) -> Result<Value, String> {
    let mut fields = vec![
        format!("atom={}", world.atom),
        format!("plumb={}", world.plumb),
        format!("runner={}", plan.runner),
    ];
    if let Some(release) = plan.release {
        fields.push(format!("release={release}"));
    }
    if let Some(target) = plan.target {
        fields.push(format!("target={target}"));
    }
    let graph = crate::command::workflow::plan::derive(crate::command::workflow::plan::Input {
        base: None,
        world: fields,
        identity: vec![format!("marker={}", world.marker)],
        project: plan
            .projections
            .iter()
            .map(|projection| format!("{}={projection}", plan.action))
            .collect(),
        roots: plan
            .roots
            .iter()
            .map(|root| format!("{}={root}", plan.action))
            .collect(),
        inventory: world.inventory.map(Path::to_path_buf),
        source: world.source.map(str::to_string),
        target: plumb::cli::Root {
            root: world.root.display().to_string(),
        },
    })?;
    let graph: Value = serde_json::from_str(&graph)
        .map_err(|error| format!("cannot decode {} plan: {error}", plan.action))?;
    graph["actions"]
        .as_array()
        .and_then(|actions| actions.iter().find(|held| held["name"] == plan.action))
        .cloned()
        .ok_or_else(|| format!("workflow plan omitted {}", plan.action))
}
