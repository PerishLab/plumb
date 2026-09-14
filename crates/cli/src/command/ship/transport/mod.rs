mod binding;
mod execute;
pub(in crate::command::ship) mod production;
mod promotion;
mod resolve;
mod reuse;
mod seal;
mod sources;
mod support;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

pub(in crate::command) fn completed(
    marker: &crate::command::release::ReleaseMarker,
) -> Result<bool, String> {
    let temporary = temporary()?;
    let root = checkout(marker, temporary.path())?;
    let held = crate::command::release::ReleaseMarker::recorded(&root, &marker.marker)?
        .ok_or("Ship evidence requires an independent release marker")?;
    if held.digest()? != marker.digest()? {
        return Err("release marker changed while reading Ship evidence".into());
    }
    promotion::completed(&held)
}

pub(super) fn local(raw: &str, dry: bool) -> Result<String, String> {
    let marker = crate::command::release::snapshot(raw)?;
    let digest = marker.digest()?;
    let mut executed = BTreeSet::new();
    loop {
        let graph: Value =
            serde_json::from_str(&inspect(raw, &digest)?).map_err(|error| error.to_string())?;
        let rows = requests(&graph)?;
        if dry || rows.is_empty() {
            return Ok(json!({
                "schema":"plumb.ship-local/v1", "marker":raw,
                "complete":graph["workload_missing"] == false && graph["publication_missing"] == false,
                "executed":executed, "graph":graph,
            }).to_string());
        }
        let mut progress = false;
        let mut pending = Vec::new();
        for row in rows {
            let request = &row["request"];
            let action = text(request, "action")?;
            let identity = format!("{action}:{}", request["keys"]);
            if !compatible(text(row, "runner")?)? || executed.contains(&identity) {
                pending.push(action.to_string());
                continue;
            }
            let latest = crate::command::release::snapshot(raw)?;
            if latest.digest()? != digest {
                return Err("release marker changed during local Ship".into());
            }
            node(&marker, request)?;
            executed.insert(identity);
            progress = true;
        }
        if !progress {
            return Err(format!(
                "local Ship remains incomplete; pending or not yet visible: {}",
                pending.join(", ")
            ));
        }
    }
}

fn requests(graph: &Value) -> Result<Vec<&Value>, String> {
    let phase = if graph["workload_missing"] == true {
        "workload"
    } else {
        "publication"
    };
    graph[phase]["include"]
        .as_array()
        .ok_or_else(|| "Ship graph has no request matrix".to_string())
        .map(|rows| {
            rows.iter()
                .filter(|row| row.get("request").is_some())
                .collect()
        })
}

fn compatible(runner: &str) -> Result<bool, String> {
    let rules = crate::catalog::set::read("workflow")?;
    let runner = if runner == "docker" { "linux" } else { runner };
    let platform = rules
        .get("execution")
        .and_then(|held| held.get("platform"))
        .and_then(|held| held.get(runner))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| format!("no execution platform for runner {runner}"))?;
    Ok(platform == plumb::config::platform())
}

fn node(marker: &crate::command::release::ReleaseMarker, request: &Value) -> Result<(), String> {
    let temporary = temporary()?;
    let root = checkout(marker, temporary.path())?;
    let status =
        Command::new(plumb::config::binary().ok_or("cannot locate the executing Plumb binary")?)
            .current_dir(&root)
            .args(["ship", "execute", "--request", &request.to_string()])
            .env("PLUMB_RELEASE_ROOT", &root)
            .env(
                "PLUMB_HOME",
                temporary
                    .path()
                    .parent()
                    .and_then(|path| path.parent())
                    .ok_or("local Ship has no home")?,
            )
            .env("PLUMB_RELEASE_VERSION", &marker.version)
            .env("PLUMB_RELEASE_CHANNEL", &marker.channel)
            .env("PLUMB_RELEASE_COMMIT", &marker.commit)
            .env(
                "PLUMB_RELEASE_ARTIFACTS",
                temporary.path().join("artifacts"),
            )
            .env("PLUMB_RELEASE_OUTPUT", temporary.path().join("output"))
            .env(
                "PLUMB_RELEASE_CAPSULE",
                temporary.path().join("output/capsule.json"),
            )
            .env(
                "PLUMB_RELEASE_PROMOTION",
                temporary.path().join("promotion.json"),
            )
            .status()
            .map_err(|error| format!("cannot execute local Ship node: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "local Ship node {} failed; repeat the same marker to resume",
            text(request, "action")?
        ))
    }
}

fn checkout(
    marker: &crate::command::release::ReleaseMarker,
    temporary: &std::path::Path,
) -> Result<PathBuf, String> {
    let root = temporary.join("source");
    let source = &marker.spec().root;
    let remote = git(Command::new("git")
        .arg("-C")
        .arg(source)
        .args(["remote", "get-url", "origin"]))?;
    git(Command::new("git")
        .args(["clone", "--local", "--no-hardlinks", "--no-checkout"])
        .arg(source)
        .arg(&root))?;
    git(Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["remote", "set-url", "origin", &remote]))?;
    git(Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["checkout", "--detach", &marker.commit]))?;
    Ok(root)
}

fn temporary() -> Result<tempfile::TempDir, String> {
    let root = plumb::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))
        .ok_or("local Ship requires PLUMB_HOME")?
        .join("tmp");
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    tempfile::Builder::new()
        .prefix("ship-")
        .tempdir_in(root)
        .map_err(|error| error.to_string())
}

fn git(command: &mut Command) -> Result<String, String> {
    let output = command.output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "cannot prepare local Ship checkout: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("Ship request has no {key}"))
}

pub(super) fn inspect(raw: &str, expected: &str) -> Result<String, String> {
    let governance = binding::Governance::resolve(raw)?;
    if governance.marker().digest()? != expected {
        return Err("release marker changed during local Ship".into());
    }
    let mut rig = plumb::rig::Rig::resolve(None).map_err(|error| error.to_string())?;
    governance.apply(&mut rig.release)?;
    resolve::graph(governance.marker(), false)
}

pub(super) fn execute(request: &str) -> Result<String, String> {
    execute::run(request)
}

pub(super) fn resolve(raw: &str, atom: &str) -> Result<String, String> {
    if atom.len() != 40 || !atom.bytes().all(|held| held.is_ascii_hexdigit()) {
        return Err("--atom must be one full Git commit".into());
    }
    let marker = crate::command::release::snapshot(raw)?;
    promotion::verify(&marker)?;
    resolve::graph(&marker, false)
}
