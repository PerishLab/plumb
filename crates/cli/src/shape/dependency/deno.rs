use super::{Dependencies, Dependency, Ecosystem};
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

pub fn read(root: &Path, scope: &str) -> Dependencies {
    let mut found = Dependencies::default();
    let runseal = root.join(".runseal/deno.json");
    if runseal.is_file() {
        found.extend(inspect(
            root,
            &runseal,
            &root.join(".runseal/deno.lock"),
            scope,
        ));
    }

    let config = root.join("deno.json");
    if !config.is_file() {
        return found;
    }
    found.extend(inspect(root, &config, &root.join("deno.lock"), scope));
    let document = match document(&config) {
        Ok(document) => document,
        Err(error) => {
            found.blind.push(error);
            return found;
        }
    };
    let Some(workspace) = document.get("workspace") else {
        return found;
    };
    let Some(members) = workspace.as_array() else {
        found.blind.push(format!(
            "cannot read {}: workspace is not an array",
            seat(root, &config)
        ));
        return found;
    };
    for member in members {
        let Some(member) = member.as_str() else {
            found.blind.push(format!(
                "cannot read {}: workspace member is not a string",
                seat(root, &config)
            ));
            continue;
        };
        let Some(path) = locate(root, member) else {
            found.blind.push(format!(
                "cannot read {}: workspace member escapes the repository: {member}",
                seat(root, &config)
            ));
            continue;
        };
        let config = path.join("deno.json");
        if !config.is_file() {
            found.blind.push(format!(
                "cannot read Deno workspace member: {} is missing",
                seat(root, &config)
            ));
            continue;
        }
        found.extend(inspect(root, &config, &root.join("deno.lock"), scope));
    }
    found
}

fn inspect(repo: &Path, config: &Path, lock: &Path, scope: &str) -> Dependencies {
    let mut found = Dependencies::default();
    let source = match document(config) {
        Ok(document) => document,
        Err(error) => {
            found.blind.push(error);
            return found;
        }
    };
    let Some(imports) = source.get("imports") else {
        return found;
    };
    let Some(imports) = imports.as_object() else {
        found.blind.push(format!(
            "cannot read {}: imports is not an object",
            seat(repo, config)
        ));
        return found;
    };
    let mut declarations = Vec::new();
    for (alias, value) in imports {
        let Some(value) = value.as_str() else {
            if alias.starts_with(&format!("{scope}/")) {
                found.blind.push(format!(
                    "cannot read {}: first-party import {alias} is not a string",
                    seat(repo, config)
                ));
            }
            continue;
        };
        if let Some(declaration) = plumb_cli::specifier(value, scope) {
            declarations.push(declaration);
        }
    }
    if declarations.is_empty() {
        return found;
    }
    let frozen = match document(lock) {
        Ok(document) => document,
        Err(error) => {
            found.blind.push(error);
            return found;
        }
    };
    let Some(specifiers) = frozen.get("specifiers").and_then(Value::as_object) else {
        found.blind.push(format!(
            "cannot read {}: specifiers is not an object",
            seat(repo, lock)
        ));
        return found;
    };
    for (name, requirement, requirement_explicit) in declarations {
        let key = format!("jsr:{name}@{requirement}");
        let mut resolutions = specifiers
            .iter()
            .filter(|(held, value)| {
                (**held == key || held.starts_with(&format!("{key}/"))) && value.is_string()
            })
            .filter_map(|(_, value)| value.as_str())
            .map(str::to_string)
            .collect::<Vec<_>>();
        resolutions.sort();
        resolutions.dedup();
        let [resolution] = resolutions.as_slice() else {
            found.blind.push(format!(
                "cannot read {}: {name} has {} matching resolutions for {key}",
                seat(repo, lock),
                resolutions.len()
            ));
            continue;
        };
        found.held.push(Dependency {
            ecosystem: Ecosystem::Jsr,
            name,
            requirement,
            pinned: requirement_explicit,
            resolution: resolution.clone(),
            latest: None,
            seat: seat(repo, config),
        });
    }
    found
}

fn document(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("cannot read {}: invalid JSON: {error}", path.display()))
}

fn locate(base: &Path, member: &str) -> Option<PathBuf> {
    let held = Path::new(member);
    if held.is_absolute()
        || held
            .components()
            .any(|component| !matches!(component, Component::CurDir | Component::Normal(_)))
    {
        return None;
    }
    Some(base.join(held))
}

fn seat(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .unwrap_or(path)
        .display()
        .to_string()
}
