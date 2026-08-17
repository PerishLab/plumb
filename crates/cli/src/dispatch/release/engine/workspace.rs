use super::super::model::Spec;
use semver::Version;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize)]
pub struct Workspace {
    packages: Vec<Package>,
    target_directory: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    version: String,
    manifest_path: PathBuf,
    targets: Vec<Target>,
}

#[derive(Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
}

pub struct Build<'a> {
    pub triple: &'a str,
    pub version: &'a str,
    pub channel: &'a str,
    pub commit: &'a str,
}

impl Workspace {
    pub fn seats(&self, root: &Path) -> BTreeMap<String, PathBuf> {
        self.packages
            .iter()
            .filter_map(|package| {
                let seat = package.manifest_path.parent()?;
                let held = seat.strip_prefix(root).unwrap_or(seat);
                Some((package.name.clone(), held.to_path_buf()))
            })
            .collect()
    }

    pub fn read(root: &Path) -> Result<Self, String> {
        let output = Command::new("cargo")
            .args(["metadata", "--no-deps", "--format-version", "1"])
            .current_dir(root)
            .output()
            .map_err(|error| format!("cannot run cargo metadata: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "cargo metadata failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("cannot parse cargo metadata: {error}"))
    }

    pub fn build(
        &self,
        spec: &Spec,
        input: Build<'_>,
    ) -> Result<BTreeMap<String, PathBuf>, String> {
        let expected = release(input.version)?;
        let mut found = BTreeMap::new();
        for binary in &spec.binaries {
            let packages = self
                .packages
                .iter()
                .filter(|package| {
                    package.targets.iter().any(|target| {
                        target.name == *binary && target.kind.iter().any(|k| k == "bin")
                    })
                })
                .collect::<Vec<_>>();
            if packages.len() != 1 {
                return Err(format!(
                    "binary {binary} belongs to {} Cargo packages",
                    packages.len()
                ));
            }
            let package = packages[0];
            let held = Version::parse(&package.version)
                .map_err(|error| format!("invalid Cargo version for {}: {error}", package.name))?;
            if (held.major, held.minor, held.patch)
                != (expected.major, expected.minor, expected.patch)
            {
                return Err(format!(
                    "release {} does not match Cargo package {} {}",
                    input.version, package.name, package.version
                ));
            }
            let msvc = input.triple.ends_with("-msvc");
            let mut command = Command::new("cargo");
            command
                .arg(if msvc { "rustc" } else { "build" })
                .args([
                    "--release",
                    "--locked",
                    "--target",
                    input.triple,
                    "--package",
                    &package.name,
                    "--bin",
                    binary,
                ])
                .current_dir(&spec.root)
                .env(
                    format!("{}_BUILD_VERSION", spec.environment()),
                    input.version,
                )
                .env(
                    format!("{}_BUILD_CHANNEL", spec.environment()),
                    input.channel,
                )
                .env(
                    format!("{}_BUILD_AUTHORITY", spec.environment()),
                    &spec.authority,
                )
                .env(format!("{}_BUILD_COMMIT", spec.environment()), input.commit);
            if msvc {
                command.args(["--", "-C", "link-arg=/Brepro"]);
            }
            let status = command
                .status()
                .map_err(|error| format!("cannot build {binary}: {error}"))?;
            if !status.success() {
                return Err(format!(
                    "cargo build failed for {binary} on {}",
                    input.triple
                ));
            }
            let suffix = if input.triple.contains("windows") {
                ".exe"
            } else {
                ""
            };
            let path = self
                .target_directory
                .join(input.triple)
                .join("release")
                .join(format!("{binary}{suffix}"));
            if !path.is_file() {
                return Err(format!("built binary is absent: {}", path.display()));
            }
            found.insert(binary.clone(), path);
        }
        Ok(found)
    }

    pub fn package(&self, name: &str) -> Result<(&Path, &str), String> {
        self.packages
            .iter()
            .find(|package| package.name == name)
            .map(|package| (package.manifest_path.as_path(), package.version.as_str()))
            .ok_or_else(|| format!("Cargo attachment package is absent: {name}"))
    }
}

pub fn release(value: &str) -> Result<Version, String> {
    let raw = value
        .strip_prefix('v')
        .ok_or_else(|| format!("release version must begin with v: {value}"))?;
    Version::parse(raw).map_err(|error| format!("invalid release version: {error}"))
}
