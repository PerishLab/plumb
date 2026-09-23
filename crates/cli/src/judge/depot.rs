use super::finding::{Finding, Seed};
use crate::catalog::rules::config::PLUMB_CURRENT;
use std::process::Command;

pub fn runtime() -> Vec<Finding> {
    let Some(manager) = managed() else {
        return Vec::new();
    };
    let running = plumb::version!("PLUMB");
    let state = match std::fs::read_to_string(&manager) {
        Ok(state) => state,
        Err(error) => {
            return vec![Finding::new(Seed::blind(
                &PLUMB_CURRENT,
                format!(
                    "cannot read managed Plumb identity {}: {error}",
                    manager.display()
                ),
            ))];
        }
    };
    let field = |name: &str| {
        state
            .lines()
            .find_map(|line| line.split_once('=').filter(|(key, _)| *key == name))
            .map(|(_, value)| value.trim())
    };
    if field("channel") != Some("stable") || field("version") != Some(running) {
        return vec![Finding::new(Seed::wrong(
            &PLUMB_CURRENT,
            format!("managed Plumb identity is not stable {running}; reinstall stable latest"),
        ))];
    }
    let releases = plumb::rig::Rig::resolve(None)
        .map(|rig| rig.releases)
        .unwrap_or_else(|_| "https://releases.plumb.perish.uk".to_string());
    let url = format!("{}/v1/channels/stable.json", releases.trim_end_matches('/'));
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--connect-timeout",
            "5",
            "--max-time",
            "15",
        ])
        .arg(&url)
        .output();
    let output = match output {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            return vec![Finding::new(Seed::blind(
                &PLUMB_CURRENT,
                format!(
                    "cannot read Plumb stable latest at {url}: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            ))];
        }
        Err(error) => {
            return vec![Finding::new(Seed::blind(
                &PLUMB_CURRENT,
                format!("cannot read Plumb stable latest at {url}: {error}"),
            ))];
        }
    };
    let pointer: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(pointer) => pointer,
        Err(error) => {
            return vec![Finding::new(Seed::blind(
                &PLUMB_CURRENT,
                format!("cannot parse Plumb stable latest at {url}: {error}"),
            ))];
        }
    };
    let latest = pointer
        .get("releaseVersion")
        .and_then(serde_json::Value::as_str);
    let valid = pointer.get("schema").and_then(serde_json::Value::as_u64) == Some(1)
        && pointer.get("product").and_then(serde_json::Value::as_str) == Some("plumb")
        && pointer.get("channel").and_then(serde_json::Value::as_str) == Some("stable");
    if !valid || latest.is_none() {
        return vec![Finding::new(Seed::blind(
            &PLUMB_CURRENT,
            format!("Plumb stable latest at {url} has an unknown shape"),
        ))];
    }
    if latest == Some(running) {
        Vec::new()
    } else {
        vec![Finding::new(Seed::wrong(
            &PLUMB_CURRENT,
            format!(
                "the running Plumb is {running}, stable latest is {}; reinstall stable latest",
                latest.unwrap_or_default()
            ),
        ))]
    }
}

fn managed() -> Option<std::path::PathBuf> {
    let executable = plumb::config::binary()?;
    let marker = executable.parent()?.join(".plumb-manager");
    marker.is_file().then_some(marker)
}
