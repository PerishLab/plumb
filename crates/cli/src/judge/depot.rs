use super::catalog::rules::depot::{DEPOT_PUBLISHED, DEPOT_SCHEMA};
use super::finding::{Finding, Seed};
use crate::dispatch::depot::record::{Manifest, Object, inventory};
use plumb::snapshot::{Refusal, Snapshot};
use semver::Version;
use std::collections::BTreeMap;

pub fn judge(snapshot: &Result<Snapshot, Refusal>) -> Vec<Finding> {
    let manifest = match crate::dispatch::depot::manifest() {
        Ok(None) => return Vec::new(),
        Ok(Some(manifest)) => manifest,
        Err(error) => return vec![Finding::new(Seed::blind(&DEPOT_PUBLISHED, error))],
    };
    let mut found = floor(&manifest);
    let Ok(snapshot) = snapshot else {
        return found;
    };
    match inventory(snapshot) {
        Ok((objects, _)) if objects.is_empty() => found,
        Ok((objects, _)) => {
            found.extend(compare(&objects, &manifest));
            found
        }
        Err(error) => {
            found.push(Finding::new(Seed::blind(&DEPOT_PUBLISHED, error)));
            found
        }
    }
}

fn floor(manifest: &Manifest) -> Vec<Finding> {
    let running = plumb::version!("PLUMB");
    let declared = &manifest.schema.version;
    let (Some(held), Some(least)) = (parse(running), parse(declared)) else {
        return vec![Finding::new(Seed::blind(
            &DEPOT_SCHEMA,
            format!("cannot compare a running {running} with a depot floor of {declared}"),
        ))];
    };
    if held >= least {
        return Vec::new();
    }
    vec![Finding::new(Seed::wrong(
        &DEPOT_SCHEMA,
        format!(
            "the held depot {} declares a floor of {declared}, above the running {running}",
            manifest.metadata.version
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
                manifest.metadata.version, object.path
            ),
            Some(sha256) if *sha256 == object.sha256 => continue,
            Some(_) => format!(
                "the held depot {} carries a different {}",
                manifest.metadata.version, object.path
            ),
        };
        found.push(Finding::new(Seed::wrong(&DEPOT_PUBLISHED, evidence)));
    }
    for path in carried.keys() {
        if !objects.iter().any(|object| object.path == *path) {
            found.push(Finding::new(Seed::wrong(
                &DEPOT_PUBLISHED,
                format!(
                    "the held depot {} carries {path}, which this repository no longer records",
                    manifest.metadata.version
                ),
            )));
        }
    }
    found
}

fn parse(version: &str) -> Option<Version> {
    Version::parse(version.trim_start_matches('v')).ok()
}
