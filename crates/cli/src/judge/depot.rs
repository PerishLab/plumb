use super::finding::{Finding, Seed};
use crate::catalog::rules::depot::{DEPOT_PUBLISHED, DEPOT_SCHEMA};
use crate::shape::depot::{Evidence, Manifest, Object};
use semver::Version;
use std::collections::BTreeMap;

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

fn floor(manifest: &Manifest) -> Vec<Finding> {
    let running = plumb::version!("PLUMB");
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
