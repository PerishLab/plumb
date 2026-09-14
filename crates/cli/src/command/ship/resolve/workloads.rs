use super::super::binding::Binding;
use super::super::support::{idle, matrix, projection, sources, text};
use super::plan::{Plan, planned};
use super::{Workloads, World};
use crate::command::release;
use serde_json::{Value, json};
pub(super) fn run(
    spec: &crate::shape::release::Spec,
    marker: &release::ReleaseMarker,
    world: &World<'_>,
) -> Result<Workloads, String> {
    if !spec.binary() {
        return Ok(Workloads {
            matrix: idle(),
            missing: false,
            reuse: Vec::new(),
        });
    }
    let targets: Value =
        serde_json::from_str(&crate::command::ship::package::product(spec).matrix()?)
            .map_err(|error| format!("cannot decode binary matrix: {error}"))?;
    let projection = projection();
    let roots = sources(spec)?;
    let listed = roots.iter().map(String::as_str).collect::<Vec<_>>();
    let mut pending = Vec::new();
    let mut reuse = Vec::new();
    let base = marker.base();
    for target in targets["include"]
        .as_array()
        .ok_or("binary matrix has no include array")?
    {
        let triple = text(target, "target")?;
        let runner = text(target, "runner")?;
        let archive = text(target, "archive")?;
        let action = format!("ship/binary.{triple}");
        let bound = planned(
            world,
            Plan {
                stage: "identity/v1",
                action: &action,
                projections: &[projection.as_str()],
                roots: &listed,
                runner,
                workload: Some(&marker.marker),
                release: Some(&marker.marker),
                target: Some(triple),
            },
        )?;
        if bound["reuse"]["type"] == "workload" {
            reuse.push(
                json!({"target": triple, "archive": archive, "url": bound["reuse"]["source"]}),
            );
            continue;
        }
        let node = planned(
            world,
            Plan {
                stage: "content/v1",
                action: &action,
                projections: &[projection.as_str()],
                roots: &listed,
                runner,
                workload: Some(base),
                release: Some(base),
                target: Some(triple),
            },
        )?;
        let build = json!({"reuse": node["reuse"], "keys": node["keys"]});
        let request = Binding::new(spec).apply(json!({
            "schema": "plumb.ship-request/v2",
            "action": action,
            "projections": [projection],
            "roots": roots,
            "operation": {
                "type": "bind",
                "build": build,
                "target": triple,
                "archive": archive,
            },
            "reuse": bound["reuse"],
            "keys": bound["keys"],
        }));
        pending.push(json!({ "runner": runner, "request": request }));
    }
    let missing = !pending.is_empty();
    Ok(Workloads {
        matrix: matrix(pending),
        missing,
        reuse,
    })
}
