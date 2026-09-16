mod binding;
#[path = "../request/controllers.rs"]
mod controllers;
#[path = "../request/declaration.rs"]
mod declaration;
#[path = "../request/execute.rs"]
mod execute;
#[path = "../request/media.rs"]
mod media;
#[path = "../request/model.rs"]
mod model;
pub(in crate::command::ship) mod production;
mod promotion;
mod receipt;
mod sources;
mod support;
use serde_json::{Value, json};
#[path = "../request/local.rs"]
mod local;

pub(in crate::command) use receipt::completed;

pub(super) fn dispatch(marker: &crate::command::release::ReleaseMarker) -> Result<Value, String> {
    let atom =
        plumb::commit!("PLUMB").ok_or("dispatch requires exact controller source identity")?;
    promotion::verify(marker)?;
    let workflow = crate::catalog::set::read("workflow")?;
    let backend = workflow
        .get("backend")
        .ok_or("locked workflow declares no finite backend")?;
    Ok(
        json!({"schema": "plumb.ship-dispatch/v1", "marker": marker.marker,
        "commit": marker.commit, "graph": declaration::graph(marker, atom)?, "backend": backend}),
    )
}

pub(super) fn local(raw: &str, dry: bool) -> Result<String, String> {
    local::run(raw, dry)
}

pub(super) fn execute(request: &str) -> Result<String, String> {
    execute::run(request)
}

pub(super) fn resolve(raw: &str, atom: &str) -> Result<String, String> {
    if atom.len() != 40 || !atom.bytes().all(|held| held.is_ascii_hexdigit()) {
        return Err("--atom must be one full Git commit".into());
    }
    let marker = crate::command::release::snapshot(raw)?;
    marker.spec().ship()?;
    promotion::verify(&marker)?;
    serde_json::to_string(&declaration::graph(&marker, atom)?).map_err(|error| error.to_string())
}
