use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn run(raw: &str, dry: bool) -> Result<String, String> {
    let governance = super::binding::Governance::resolve(raw)?;
    let marker = governance.marker();
    let declaration = super::dispatch(marker)?;
    if dry {
        return serde_json::to_string(&declaration).map_err(|error| error.to_string());
    }
    let mut rig = plumb::rig::Rig::resolve(None).map_err(|error| error.to_string())?;
    governance.apply(&mut rig.release)?;
    rig.workflow.inventory.load()?;
    let seat = temporary()?;
    let control = control(&marker.spec().root, &seat)?;
    let input = seat.join("declaration.json");
    std::fs::write(&input, declaration.to_string()).map_err(|error| error.to_string())?;
    let inventory = &rig.workflow.inventory;
    let python = if cfg!(windows) { "python" } else { "python3" };
    let status = Command::new(python)
        .arg(control.join(".forgejo/scripts/local.py"))
        .arg("--declaration")
        .arg(input)
        .arg("--control")
        .arg(&control)
        .arg("--root")
        .arg(&marker.spec().root)
        .args([
            "--runners",
            &serde_json::to_string(&runners()?).map_err(|error| error.to_string())?,
        ])
        .current_dir(&marker.spec().root)
        .env("RUNNER_TEMP", &seat)
        .env("PLUMB_RELEASE_ROOT", ".")
        .env("PLUMB_RELEASE_MARKER", &marker.marker)
        .env("PLUMB_WORKFLOW_INVENTORY_ACCESS", &inventory.access)
        .env("PLUMB_WORKFLOW_INVENTORY_SECRET", &inventory.secret)
        .env("PLUMB_WORKFLOW_INVENTORY_BUCKET", &inventory.bucket)
        .env("PLUMB_WORKFLOW_INVENTORY_ENDPOINT", &inventory.endpoint)
        .env("PLUMB_WORKFLOW_INVENTORY_URL", &inventory.url)
        .status()
        .map_err(|error| format!("cannot run central local backend: {error}"))?;
    if crate::command::release::snapshot(raw)?.digest()? != marker.digest()? {
        return Err("release marker changed during local Ship".into());
    }
    if !status.success() {
        return Err(format!(
            "local Ship remains incomplete; retained execution state at {}",
            seat.display()
        ));
    }
    Ok(format!(
        "local Ship completed {raw}; execution state retained at {}",
        seat.display()
    ))
}

fn runners() -> Result<Vec<String>, String> {
    let workflow = crate::catalog::set::read("workflow")?;
    let platforms = workflow
        .get("execution")
        .and_then(|value| value.get("platform"))
        .and_then(toml::Value::as_table)
        .ok_or("workflow declares no runner platforms")?;
    let host = plumb::config::platform();
    Ok(platforms
        .iter()
        .filter(|(_, platform)| platform.as_str() == Some(host.as_str()))
        .map(|(runner, _)| runner.clone())
        .collect())
}

fn control(root: &Path, seat: &Path) -> Result<PathBuf, String> {
    let remote = plumb::forgejo::git::remote(root, "PerishLab/plumb")?;
    let address = format!(
        "{}://{}/{}/{}.git",
        remote.scheme, remote.host, remote.owner, remote.repo
    );
    let commit = plumb::commit!("PLUMB").ok_or("local Ship needs an exact controller source")?;
    let control = seat.join("control");
    git(Command::new("git").args(["init", "--quiet"]).arg(&control))?;
    git(Command::new("git")
        .arg("-C")
        .arg(&control)
        .args(["remote", "add", "origin", &address]))?;
    git(Command::new("git").arg("-C").arg(&control).args([
        "fetch",
        "--depth=1",
        "origin",
        commit,
    ]))?;
    git(Command::new("git")
        .arg("-C")
        .arg(&control)
        .args(["checkout", "--detach", "FETCH_HEAD"]))?;
    Ok(control)
}

fn temporary() -> Result<PathBuf, String> {
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
        .map(tempfile::TempDir::keep)
        .map_err(|error| error.to_string())
}

fn git(command: &mut Command) -> Result<(), String> {
    let output = command.output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "cannot prepare central control source: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}
