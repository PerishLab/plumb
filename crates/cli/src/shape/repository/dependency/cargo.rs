use super::{Dependencies, Dependency, Ecosystem};
use semver::{Version, VersionReq};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    #[serde(rename = "manifest_path")]
    manifest: std::path::PathBuf,
    dependencies: Vec<Declared>,
}

#[derive(Deserialize)]
struct Declared {
    name: String,
    req: String,
    source: Option<String>,
}

#[derive(Deserialize)]
struct Lock {
    #[serde(default)]
    package: Vec<Locked>,
}

#[derive(Deserialize)]
struct Locked {
    name: String,
    version: String,
    source: Option<String>,
}

pub fn read(root: &Path, registry: &str, index: &str) -> Dependencies {
    let mut found = Dependencies::default();
    let (candidates, blind) = declared(root, registry);
    found.blind.extend(blind);
    if !root.join("Cargo.toml").is_file() || candidates.is_empty() {
        return found;
    }
    let output = match Command::new("cargo")
        .args(["metadata", "--frozen", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            found
                .blind
                .push(format!("cannot read Cargo dependencies: {error}"));
            return found;
        }
    };
    if !output.status.success() {
        found.blind.push(format!(
            "cannot read Cargo dependencies: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
        return found;
    }
    let metadata: Metadata = match serde_json::from_slice(&output.stdout) {
        Ok(metadata) => metadata,
        Err(error) => {
            found
                .blind
                .push(format!("cannot read Cargo metadata: {error}"));
            return found;
        }
    };
    let mut declarations = Vec::new();
    for package in metadata.packages {
        for dependency in package.dependencies {
            if !candidates.contains(&dependency.name) {
                continue;
            }
            if dependency.source.as_deref() == Some(index) {
                declarations.push((package.manifest.clone(), dependency.name, dependency.req));
            } else if dependency.source.is_some() {
                found.blind.push(format!(
                    "cannot read Cargo registry {registry} for {} in {}: source is {}",
                    dependency.name,
                    seat(root, &package.manifest),
                    dependency.source.as_deref().unwrap_or_default(),
                ));
            }
        }
    }
    if declarations.is_empty() {
        return found;
    }
    let path = root.join("Cargo.lock");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            found.blind.push(format!("cannot read Cargo.lock: {error}"));
            return found;
        }
    };
    let lock: Lock = match toml::from_str(&text) {
        Ok(lock) => lock,
        Err(error) => {
            found.blind.push(format!("cannot read Cargo.lock: {error}"));
            return found;
        }
    };
    for (manifest, name, requirement) in declarations {
        let parsed = match VersionReq::parse(&requirement) {
            Ok(parsed) => parsed,
            Err(error) => {
                found.blind.push(format!(
                    "cannot read Cargo requirement {requirement} for {name}: {error}"
                ));
                continue;
            }
        };
        let mut resolutions = lock
            .package
            .iter()
            .filter(|held| held.name == name && held.source.as_deref() == Some(index))
            .filter_map(|held| {
                Version::parse(&held.version)
                    .ok()
                    .filter(|version| parsed.matches(version))
                    .map(|_| held.version.clone())
            })
            .collect::<Vec<_>>();
        resolutions.sort();
        resolutions.dedup();
        let [resolution] = resolutions.as_slice() else {
            found.blind.push(format!(
                "cannot read Cargo.lock: {name} has {} resolutions matching {requirement}",
                resolutions.len()
            ));
            continue;
        };
        found.held.push(Dependency {
            ecosystem: Ecosystem::Cargo,
            name,
            requirement,
            pinned: false,
            resolution: resolution.clone(),
            latest: None,
            seat: seat(root, &manifest),
        });
    }
    found
}

fn declared(repo: &Path, registry: &str) -> (std::collections::BTreeSet<String>, Vec<String>) {
    let mut found = std::collections::BTreeSet::new();
    let mut blind = Vec::new();
    for path in manifests(repo) {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                blind.push(format!(
                    "cannot read Cargo manifest {}: {error}",
                    seat(repo, &path)
                ));
                continue;
            }
        };
        let document = match parse(&text, registry) {
            Ok(Some(document)) => document,
            Ok(None) => continue,
            Err(error) => {
                blind.push(format!(
                    "cannot read first-party Cargo manifest {}: {error}",
                    seat(repo, &path)
                ));
                continue;
            }
        };
        found.extend(plumb_cli::packages(&document, registry));
    }
    (found, blind)
}

fn parse(text: &str, registry: &str) -> Result<Option<toml::Value>, toml::de::Error> {
    match toml::from_str(text) {
        Ok(document) => Ok(Some(document)),
        Err(error) if text.contains("registry") && text.contains(registry) => Err(error),
        Err(_) => Ok(None),
    }
}

fn manifests(base: &Path) -> Vec<std::path::PathBuf> {
    let mut found = vec![base.join("Cargo.toml"), base.join("app/Cargo.toml")];
    if let Ok(entries) = std::fs::read_dir(base.join("crates")) {
        for entry in entries.flatten() {
            found.push(entry.path().join("Cargo.toml"));
        }
    }
    found.retain(|path| path.is_file());
    found
}

fn seat(repo: &Path, path: &Path) -> String {
    if let Ok(repo) = repo.canonicalize()
        && let Ok(path) = path.strip_prefix(repo)
    {
        return path.display().to_string();
    }
    path.strip_prefix(repo)
        .unwrap_or(path)
        .display()
        .to_string()
}
