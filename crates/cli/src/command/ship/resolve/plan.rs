use super::World;
use serde_json::Value;
use std::path::Path;
pub(super) struct Plan<'a> {
    pub(super) stage: &'static str,
    pub(super) action: &'a str,
    pub(super) projections: &'a [&'a str],
    pub(super) roots: &'a [&'a str],
    pub(super) runner: &'a str,
    pub(super) workload: Option<&'a str>,
    pub(super) release: Option<&'a str>,
    pub(super) target: Option<&'a str>,
}
pub(super) fn planned(world: &World<'_>, plan: Plan<'_>) -> Result<Value, String> {
    let mut fields = vec![
        format!("runner={}", plan.runner),
        format!("phase={}", plan.stage),
    ];
    if let Some(release) = plan.release {
        fields.push(format!("release={release}"));
    }
    if let Some(target) = plan.target {
        fields.push(format!("target={target}"));
    }
    let workload = if plan.stage == "publication" {
        plan.workload
            .map(|value| vec![format!("release={value}")])
            .unwrap_or_default()
    } else {
        fields.clone()
    };
    let named = |values: &[&str]| {
        values
            .iter()
            .map(|value| format!("{}={value}", plan.action))
            .collect()
    };
    let graph = crate::command::workflow::plan::derive(
        crate::command::workflow::plan::Input {
            base: None,
            world: fields,
            workload,
            identity: world.binding.identity(world.marker),
            project: named(plan.projections),
            roots: named(plan.roots),
            inventory: world.inventory.map(Path::to_path_buf),
            source: world.source.map(str::to_string),
            target: plumb::cli::Root {
                root: world.root.display().to_string(),
            },
        },
        Some(plan.action),
    )?;
    let graph: Value = serde_json::from_str(&graph)
        .map_err(|error| format!("cannot decode {} plan: {error}", plan.action))?;
    graph["actions"]
        .as_array()
        .and_then(|actions| actions.iter().find(|held| held["name"] == plan.action))
        .cloned()
        .ok_or_else(|| format!("workflow plan omitted {}", plan.action))
}
