use crate::command::release;
use plumb::rig::Rig;
use serde_json::{Value, json};
use std::path::Path;

use super::support::{Inventory, depot, object, projection, strings, text};

const SCHEMA: &str = "plumb.ship-graph/v1";

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
        depot: &depot(root)?,
        plumb: env!("CARGO_PKG_VERSION"),
        marker: &marker.marker,
        inventory: inventory.path(),
        root,
    };
    let binary = binary(&spec, &marker, &world)?;
    let project = projects(&plan["project"], &world)?;
    serde_json::to_string(&json!({
        "schema": SCHEMA,
        "channel": marker.channel,
        "commit": marker.commit,
        "version": marker.version,
        "targets": binary.targets,
        "binary_missing": binary.missing,
        "binary_reuse": binary.reuse,
        "project": project.matrix,
        "project_missing": project.missing,
    }))
    .map_err(|error| format!("cannot encode ship graph: {error}"))
}

struct World<'a> {
    atom: &'a str,
    depot: &'a str,
    plumb: &'a str,
    marker: &'a str,
    inventory: Option<&'a Path>,
    root: &'a Path,
}

struct Binary {
    targets: Value,
    missing: bool,
    reuse: Vec<Value>,
}

fn binary(
    spec: &crate::shape::release::Spec,
    marker: &release::ReleaseMarker,
    world: &World<'_>,
) -> Result<Binary, String> {
    let mut targets: Value = serde_json::from_str(&super::super::package::product(spec).matrix()?)
        .map_err(|error| format!("cannot decode binary matrix: {error}"))?;
    if marker.channel == "stable" {
        return Ok(Binary {
            targets,
            missing: false,
            reuse: Vec::new(),
        });
    }
    let projection = projection(world.root);
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
                roots: &[],
                runner,
                release: Some(&marker.version),
                target: Some(triple),
            },
        )?;
        if node["reuse"]["type"] == "workload" {
            reuse.push(json!({
                "runner": runner,
                "target": triple,
                "archive": archive,
                "url": node["reuse"]["source"],
            }));
        } else {
            let mut target = object(target)?;
            target.insert("action".into(), json!(action));
            target.insert("keys".into(), node["keys"].clone());
            pending.push(Value::Object(target));
        }
    }
    let missing = !pending.is_empty();
    targets = if missing {
        json!({ "include": pending })
    } else {
        json!({ "include": [{
            "runner": "docker",
            "control": "reuse",
            "target": "reuse",
            "archive": "none"
        }] })
    };
    Ok(Binary {
        targets,
        missing,
        reuse,
    })
}

struct Projects {
    matrix: Value,
    missing: bool,
}

fn projects(input: &Value, world: &World<'_>) -> Result<Projects, String> {
    let mut pending = Vec::new();
    for entry in input["include"]
        .as_array()
        .ok_or("project plan has no include array")?
    {
        let action = text(entry, "action")?;
        let projections = strings(entry, "projections")?;
        let roots = strings(entry, "roots")?;
        let node = planned(
            world,
            Plan {
                action,
                projections: &projections,
                roots: &roots,
                runner: "docker",
                release: None,
                target: None,
            },
        )?;
        if node["reuse"]["type"] == "url" {
            continue;
        }
        let mut entry = object(entry)?;
        entry.insert("reuse".into(), node["reuse"].clone());
        entry.insert("keys".into(), node["keys"].clone());
        pending.push(Value::Object(entry));
    }
    let missing = !pending.is_empty();
    let matrix = if missing {
        json!({ "include": pending })
    } else {
        json!({ "include": [{ "control": "reuse" }] })
    };
    Ok(Projects { matrix, missing })
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
        format!("depot={}", world.depot),
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
