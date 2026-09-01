use super::finding::{Finding, Seed};
use crate::catalog::rules::depot::{DEPOT_PUBLISHED, DEPOT_SCHEMA};
use crate::shape::depot::{Evidence, Manifest, Object};
use semver::Version;
use std::collections::BTreeMap;
use std::process::Command;

pub fn judge(evidence: &Evidence) -> Vec<Finding> {
    let (manifest, inventory) = match evidence {
        Evidence::Absent => return Vec::new(),
        Evidence::Blind(error) => {
            return vec![Finding::new(Seed::blind(&DEPOT_PUBLISHED, error.clone()))];
        }
        Evidence::Held {
            manifest,
            inventory,
        } => (manifest, inventory),
    };
    let mut found = floor(manifest);
    let Some(inventory) = inventory else {
        return found;
    };
    match inventory {
        Ok(objects) if objects.is_empty() => found,
        Ok(objects) => {
            found.extend(compare(objects, manifest));
            found
        }
        Err(error) => {
            found.push(Finding::new(Seed::blind(&DEPOT_PUBLISHED, error.clone())));
            found
        }
    }
}

pub fn runtime() -> Vec<Finding> {
    let Some(manager) = managed() else {
        return Vec::new();
    };
    let running = plumb::version!("PLUMB");
    let state = match std::fs::read_to_string(&manager) {
        Ok(state) => state,
        Err(error) => {
            return vec![Finding::new(Seed::blind(
                &DEPOT_SCHEMA,
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
            &DEPOT_SCHEMA,
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
                &DEPOT_SCHEMA,
                format!(
                    "cannot read Plumb stable latest at {url}: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            ))];
        }
        Err(error) => {
            return vec![Finding::new(Seed::blind(
                &DEPOT_SCHEMA,
                format!("cannot read Plumb stable latest at {url}: {error}"),
            ))];
        }
    };
    let pointer: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(pointer) => pointer,
        Err(error) => {
            return vec![Finding::new(Seed::blind(
                &DEPOT_SCHEMA,
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
            &DEPOT_SCHEMA,
            format!("Plumb stable latest at {url} has an unknown shape"),
        ))];
    }
    if latest == Some(running) {
        Vec::new()
    } else {
        vec![Finding::new(Seed::wrong(
            &DEPOT_SCHEMA,
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

fn floor(manifest: &Manifest) -> Vec<Finding> {
    let running = plumb::version!("PLUMB");
    if let Some(exact) = &manifest.version
        && parse(running) != parse(exact)
    {
        return vec![Finding::new(Seed::wrong(
            &DEPOT_SCHEMA,
            format!(
                "the held configuration {} belongs to Plumb {exact}, not the running {running}; install stable latest and run plumb configuration install",
                manifest.mark
            ),
        ))];
    }
    let declared = &manifest.floor;
    let (Some(held), Some(least)) = (parse(running), parse(declared)) else {
        return vec![Finding::new(Seed::blind(
            &DEPOT_SCHEMA,
            format!("cannot compare a running {running} with a depot floor of {declared}"),
        ))];
    };
    if plumb::depot::supports(&held, &least) {
        return Vec::new();
    }
    vec![Finding::new(Seed::wrong(
        &DEPOT_SCHEMA,
        format!(
            "the held depot {} declares a floor of {declared}, above the running {running}",
            manifest.mark
        ),
    ))]
}

fn compare(objects: &[Object], manifest: &Manifest) -> Vec<Finding> {
    let carried = manifest
        .objects
        .iter()
        .map(|object| (object.path.as_str(), object.sha256.as_str()))
        .collect::<BTreeMap<_, _>>();
    let mut found = Vec::new();
    for object in objects {
        let evidence = match carried.get(object.path.as_str()) {
            None => format!(
                "the held depot {} carries no {}",
                manifest.mark, object.path
            ),
            Some(sha256) if *sha256 == object.sha256 => continue,
            Some(_) => format!(
                "the held depot {} carries a different {}",
                manifest.mark, object.path
            ),
        };
        found.push(Finding::new(Seed::noted(&DEPOT_PUBLISHED, evidence)));
    }
    for path in carried.keys() {
        if !objects.iter().any(|object| object.path == *path) {
            found.push(Finding::new(Seed::noted(
                &DEPOT_PUBLISHED,
                format!(
                    "the held depot {} carries {path}, which this repository no longer records",
                    manifest.mark
                ),
            )));
        }
    }
    found
}

fn parse(version: &str) -> Option<Version> {
    Version::parse(version.trim_start_matches('v')).ok()
}
