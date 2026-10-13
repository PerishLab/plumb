use super::{Dependencies, Dependency, Ecosystem};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const SECTIONS: [&str; 3] = ["dependencies", "devDependencies", "optionalDependencies"];

struct Declared {
    seat: String,
    section: String,
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
        declare(&seat, &document, &mut declared, &mut found.blind);
    }
    let external = declared.iter().any(|held| !members.contains(&held.name));
    let lock = external.then(|| lock(root)).transpose();
    let lock = match lock {
        Ok(lock) => lock,
        Err(error) => {
            found.blind.push(error);
            None
        }
    };
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
        if !members.contains(&held.name)
            && let Some(lock) = &lock
        {
            match resolution(lock, &held) {
                Ok(version) => found.held.push(Dependency {
                    ecosystem: Ecosystem::Npm,
                    name: held.name,
                    pinned: held.requirement != expected,
                    requirement: held.requirement,
                    resolution: version,
                    latest: None,
                    seat: held.seat,
                }),
                Err(error) => found.blind.push(error),
            }
        }
    }
}

fn lock(root: &Path) -> Result<serde_yaml_ng::Value, String> {
    let text = std::fs::read_to_string(root.join("pnpm-lock.yaml"))
        .map_err(|error| format!("cannot read npm lock: {error}"))?;
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text)
        .map_err(|error| format!("cannot parse npm lock: {error}"))?;
    if value["lockfileVersion"].as_str() != Some("9.0") {
        return Err("unsupported npm lock format".into());
    }
    Ok(value)
}

fn resolution(lock: &serde_yaml_ng::Value, held: &Declared) -> Result<String, String> {
    let importer = held.seat.strip_suffix("/package.json").unwrap_or(".");
    let text = lock["importers"][importer][&held.section][&held.name]["version"]
        .as_str()
        .ok_or_else(|| {
            format!(
                "npm {} has no importer resolution [{}]",
                held.name, held.seat
            )
        })?;
    let version = text.split('(').next().unwrap_or_default();
    let parsed = semver::Version::parse(version)
        .map_err(|_| format!("invalid npm resolution for {} [{}]", held.name, held.seat))?;
    let key = format!("{}@{parsed}", held.name);
    if lock["packages"].get(&key).is_none() {
        return Err(format!(
            "npm {} importer has no package entry [{}]",
            held.name, held.seat
        ));
    }
    Ok(parsed.to_string())
}

fn declare(
    seat: &str,
    document: &serde_json::Value,
    declared: &mut Vec<Declared>,
    blind: &mut Vec<String>,
) {
    for section in SECTIONS {
        for (name, requirement) in document[section].as_object().into_iter().flatten() {
            let requirement = requirement.as_str().unwrap_or_default();
            if requirement.starts_with("npm:")
                && (name.starts_with("@perishlab/") || requirement.starts_with("npm:@perishlab/"))
            {
                blind.push(format!("unsupported npm alias {name} [{seat}]"));
                continue;
            }
            if name.starts_with("@perishlab/") {
                declared.push(Declared {
                    seat: seat.into(),
                    section: section.into(),
                    name: name.clone(),
                    requirement: requirement.into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("package.json"),
            r#"{"dependencies":{"@perishlab/design":"0"}}"#,
        )
        .unwrap();
        std::fs::write(root.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\nimporters:\n  .:\n    dependencies:\n      '@perishlab/design':\n        specifier: '0'\n        version: 0.3.14(svelte@5.0.0)\npackages:\n  '@perishlab/design@0.3.14': {}\n").unwrap();
        root
    }

    #[test]
    fn currency() {
        let root = fixture();
        let mut held = Dependencies::default();
        read(root.path(), &mut held);
        assert!(held.blind.is_empty(), "{:?}", held.blind);
        assert_eq!(held.held.len(), 1);
        let dependency = &mut held.held[0];
        assert_eq!(dependency.resolution, "0.3.14");
        dependency.latest = Some("0.3.14".into());
        assert!(plumb_cli::judge(dependency).is_empty());
        dependency.latest = Some("0.3.15".into());
        assert!(matches!(
            plumb_cli::judge(dependency).as_slice(),
            [plumb_cli::Verdict::Stale { .. }]
        ));
    }

    #[test]
    fn blind() {
        let root = fixture();
        for text in [
            "not: [yaml",
            "lockfileVersion: 'unknown'",
            "lockfileVersion: '9.0'\nimporters: {}",
            "lockfileVersion: '9.0'\nimporters:\n  .:\n    dependencies:\n      '@perishlab/design':\n        version: '@perishlab/alias@0.3.14'\n",
        ] {
            std::fs::write(root.path().join("pnpm-lock.yaml"), text).unwrap();
            let mut held = Dependencies::default();
            read(root.path(), &mut held);
            assert!(!held.blind.is_empty(), "{text}");
            assert!(held.held.is_empty());
        }
        std::fs::remove_file(root.path().join("pnpm-lock.yaml")).unwrap();
        let mut held = Dependencies::default();
        read(root.path(), &mut held);
        assert!(!held.blind.is_empty());
    }

    #[test]
    fn workspace() {
        let root = fixture();
        std::fs::create_dir_all(root.path().join("packages/design")).unwrap();
        std::fs::write(
            root.path().join("packages/design/package.json"),
            r#"{"name":"@perishlab/design"}"#,
        )
        .unwrap();
        std::fs::write(
            root.path().join("package.json"),
            r#"{"dependencies":{"@perishlab/design":"workspace:*"}}"#,
        )
        .unwrap();
        std::fs::remove_file(root.path().join("pnpm-lock.yaml")).unwrap();
        let mut held = Dependencies::default();
        read(root.path(), &mut held);
        assert!(held.held.is_empty() && held.blind.is_empty() && held.pinned.is_empty());
    }

    #[test]
    fn aliases() {
        let root = fixture();
        for name in ["@perishlab/alias", "ordinary-alias"] {
            let document = serde_json::json!({"dependencies":{name:"npm:@perishlab/design@0"}});
            std::fs::write(root.path().join("package.json"), document.to_string()).unwrap();
            let mut held = Dependencies::default();
            read(root.path(), &mut held);
            assert!(held.held.is_empty());
            assert!(
                held.blind
                    .iter()
                    .any(|error| error.contains("unsupported npm alias"))
            );
        }
    }
}
