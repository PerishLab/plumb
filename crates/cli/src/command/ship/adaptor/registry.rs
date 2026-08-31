use super::{ledger, manifest};
use crate::command::release::workspace::{Workspace, release};
use crate::shape::release::{Cargo, Spec};
use flate2::read::GzDecoder;
use semver::Version;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Registry<'a> {
    pub(in crate::command::ship) spec: &'a Spec,
}

pub fn registry(spec: &Spec) -> Registry<'_> {
    Registry { spec }
}

impl Registry<'_> {
    pub(in crate::command) fn prepare(&self, version: &str) -> Result<(), String> {
        if !self.spec.root.join("Cargo.toml").is_file() {
            return Ok(());
        }
        let identity = release(version)?;
        self.project(&identity)
    }

    pub fn rehearse(&self, version: &str, token: &str) -> Result<String, String> {
        let Some(cargo) = &self.spec.cargo else {
            return Ok(format!("{} has no Cargo attachment", self.spec.product));
        };
        if token.trim().is_empty() {
            return Err("PLUMB_RELEASE_REGISTRY_TOKEN is required".into());
        }
        self.stamp(version)?;
        let identity = release(version)?;
        let workspace = Workspace::read(&self.spec.root)?;
        for package in self.ordered(cargo)? {
            let (seat, _) = workspace.package(package)?;
            let held = manifest::read(seat)?;
            let siblings = cargo
                .packages
                .iter()
                .filter(|held| *held != package)
                .cloned()
                .collect::<Vec<_>>();
            if manifest::coupled(&held, &siblings) {
                println!("  {package} requires a sibling this release has not published yet");
                continue;
            }
            self.command(
                [
                    "package",
                    "--registry",
                    &cargo.registry,
                    "--package",
                    package,
                    "--allow-dirty",
                ],
                token,
            )?;
            inspect(&self.archive(package, &identity), package, &identity)?;
        }
        Ok(format!("rehearsed Cargo attachment for {version}"))
    }

    pub fn publish(&self, version: &str, token: &str) -> Result<String, String> {
        let Some(cargo) = &self.spec.cargo else {
            return Ok(format!("{} has no Cargo attachment", self.spec.product));
        };
        if token.trim().is_empty() {
            return Err("PLUMB_RELEASE_REGISTRY_TOKEN is required".into());
        }
        self.stamp(version)?;
        let identity = release(version)?;
        for package in self.ordered(cargo)? {
            self.command(
                [
                    "package",
                    "--registry",
                    &cargo.registry,
                    "--package",
                    package,
                    "--allow-dirty",
                    "--no-verify",
                ],
                token,
            )?;
            let archive = self.archive(package, &identity);
            inspect(&archive, package, &identity)?;
            let checksum = crate::command::release::record::digest(&archive)?.0;
            let held = ledger::entries(self.spec, cargo, package, token)?;
            if ledger::verified(package, &identity, &checksum, &held)? {
                continue;
            }
            self.command(
                [
                    "publish",
                    "--registry",
                    &cargo.registry,
                    "--package",
                    package,
                    "--allow-dirty",
                    "--dry-run",
                ],
                token,
            )?;
            if crate::command::release::record::digest(&archive)?.0 != checksum {
                return Err(format!(
                    "Cargo package {package} changed during publisher dry run"
                ));
            }
            self.command(
                [
                    "publish",
                    "--registry",
                    &cargo.registry,
                    "--package",
                    package,
                    "--allow-dirty",
                ],
                token,
            )?;
            ledger::readback(ledger::Readback {
                spec: self.spec,
                cargo,
                package,
                version: &identity,
                checksum: &checksum,
                token,
            })?;
        }
        Ok(format!("published Cargo attachment for {version}"))
    }

    pub(in crate::command::ship) fn ordered<'a>(
        &self,
        cargo: &'a Cargo,
    ) -> Result<&'a [String], String> {
        if cargo.packages.is_empty() {
            return Err("Cargo attachment must declare ordered packages".into());
        }
        Ok(&cargo.packages)
    }

    pub(in crate::command::ship) fn stamp(&self, version: &str) -> Result<(), String> {
        let identity = release(version)?;
        let root = self.spec.root.join("Cargo.toml");
        let document = manifest::read(&root)?;
        let base = document["workspace"]["package"]["version"]
            .as_str()
            .or_else(|| document["package"]["version"].as_str())
            .ok_or_else(|| "Cargo root has no package version".to_string())?;
        let held =
            Version::parse(base).map_err(|error| format!("invalid workspace version: {error}"))?;
        if (held.major, held.minor, held.patch) != (identity.major, identity.minor, identity.patch)
        {
            return Err(format!(
                "workspace version {held} is not release base {identity}"
            ));
        }
        self.project(&identity)
    }
    fn project(&self, identity: &Version) -> Result<(), String> {
        let lock = self.spec.root.join("Cargo.lock");
        let locked = lock.is_file();
        let workspace = Workspace::read(&self.spec.root);
        if !locked && lock.is_file() {
            std::fs::remove_file(&lock)
                .map_err(|error| format!("cannot remove generated {}: {error}", lock.display()))?;
        }
        let workspace = workspace?;
        let manifests = workspace.manifests();
        let pins = manifests
            .keys()
            .map(|name| (name.clone(), identity.to_string()))
            .collect::<BTreeMap<_, _>>();
        let root = self.spec.root.join("Cargo.toml");
        let mut paths = manifests.values().cloned().collect::<Vec<_>>();
        if !paths.contains(&root) {
            paths.push(root.clone());
        }
        paths.sort();
        paths.dedup();
        for path in paths {
            let mut document = manifest::read(&path)?;
            if path == root {
                if document["workspace"]["package"]["version"]
                    .as_str()
                    .is_some()
                {
                    document["workspace"]["package"]["version"] =
                        toml_edit::value(identity.to_string());
                } else if document["package"]["version"].as_str().is_some() {
                    document["package"]["version"] = toml_edit::value(identity.to_string());
                }
            } else if document["package"]["version"].as_str().is_some() {
                document["package"]["version"] = toml_edit::value(identity.to_string());
            }
            manifest::dependencies(&mut document, &pins)?;
            manifest::write(&path, &document)?;
        }
        self.lock(identity, &pins)
    }

    fn lock(&self, identity: &Version, pins: &BTreeMap<String, String>) -> Result<(), String> {
        let path = self.spec.root.join("Cargo.lock");
        if !path.is_file() {
            return Ok(());
        }
        let mut document = manifest::read(&path)?;
        let packages = document["package"]
            .as_array_of_tables_mut()
            .ok_or_else(|| "Cargo.lock has no package array".to_string())?;
        for package in packages.iter_mut() {
            let Some(name) = package.get("name").and_then(toml_edit::Item::as_str) else {
                continue;
            };
            if pins.contains_key(name) && !package.contains_key("source") {
                package["version"] = toml_edit::value(identity.to_string());
            }
        }
        manifest::write(&path, &document)
    }

    pub(in crate::command::ship) fn command<const N: usize>(
        &self,
        args: [&str; N],
        token: &str,
    ) -> Result<(), String> {
        let mut command = Command::new("cargo");
        command.args(args).current_dir(&self.spec.root);
        if let Some(cargo) = &self.spec.cargo
            && !token.is_empty()
        {
            command.env(
                format!(
                    "CARGO_REGISTRIES_{}_TOKEN",
                    cargo.registry.to_ascii_uppercase().replace('-', "_")
                ),
                token,
            );
        }
        let status = command
            .status()
            .map_err(|error| format!("cannot run cargo: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("Cargo attachment command failed".into())
        }
    }

    pub(in crate::command::ship) fn archive(&self, package: &str, version: &Version) -> PathBuf {
        self.spec
            .root
            .join("target/package")
            .join(format!("{package}-{version}.crate"))
    }
}

pub(in crate::command::ship) fn inspect(
    path: &Path,
    package: &str,
    version: &Version,
) -> Result<(), String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut archive = tar::Archive::new(GzDecoder::new(file));
    let wanted = format!("{package}-{version}/Cargo.toml");
    for entry in archive.entries().map_err(|error| error.to_string())? {
        let mut entry = entry.map_err(|error| error.to_string())?;
        if entry.path().map_err(|error| error.to_string())? == Path::new(&wanted) {
            let mut manifest = String::new();
            entry
                .read_to_string(&mut manifest)
                .map_err(|error| error.to_string())?;
            if manifest.contains(&format!("version = \"{version}\"")) {
                return Ok(());
            }
            return Err(format!("{wanted} does not name version {version}"));
        }
    }
    Err(format!("Cargo archive misses {wanted}"))
}
