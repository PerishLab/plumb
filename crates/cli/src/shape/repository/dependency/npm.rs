use super::Dependencies;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const SECTIONS: [&str; 3] = ["dependencies", "devDependencies", "optionalDependencies"];

struct Declared {
    seat: String,
    name: String,
    requirement: String,
}

pub fn read(root: &Path, found: &mut Dependencies) {
    let mut members = BTreeSet::new();
    let mut declared = Vec::new();
    for (seat, path) in manifests(root) {
        let document = match std::fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|text| {
                serde_json::from_str::<serde_json::Value>(&text).map_err(|error| error.to_string())
            }) {
            Ok(document) => document,
            Err(error) => {
                found
                    .illegible
                    .push(format!("cannot read npm manifest {seat}: {error}"));
                continue;
            }
        };
        if let Some(name) = document["name"].as_str() {
            members.insert(name.to_string());
        }
        declare(&seat, &document, &mut declared);
    }
    for held in declared {
        let expected = if members.contains(&held.name) {
            "workspace:*"
        } else {
            "0"
        };
        if held.requirement != expected {
            found.pinned.push(format!(
                "npm {} requires {}, expected {expected} [{}]",
                held.name, held.requirement, held.seat
            ));
        }
    }
}

fn declare(seat: &str, document: &serde_json::Value, declared: &mut Vec<Declared>) {
    for section in SECTIONS {
        for (name, requirement) in document[section].as_object().into_iter().flatten() {
            if name.starts_with("@perishlab/") {
                declared.push(Declared {
                    seat: seat.into(),
                    name: name.clone(),
                    requirement: requirement.as_str().unwrap_or_default().into(),
                });
            }
        }
    }
}

fn manifests(root: &Path) -> Vec<(String, PathBuf)> {
    let mut found = vec![("package.json".to_string(), root.join("package.json"))];
    for group in ["packages", "apps"] {
        let Ok(entries) = std::fs::read_dir(root.join(group)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            found.push((
                format!("{group}/{name}/package.json"),
                entry.path().join("package.json"),
            ));
        }
    }
    found.retain(|(_, path)| path.is_file());
    found.sort();
    found
}
