use super::super::ledger;
use super::manifest;
use crate::dispatch::release::engine::workspace::{Workspace, release};
use crate::dispatch::release::model::{Cargo, Spec};
use flate2::read::GzDecoder;
use semver::Version;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Registry<'a> {
    spec: &'a Spec,
}

pub fn registry(spec: &Spec) -> Registry<'_> {
    Registry { spec }
}

impl Registry<'_> {
    pub fn rehearse(&self, version: &str, token: &str) -> Result<String, String> {
        let Some(cargo) = &self.spec.cargo else {
            return Ok(format!("{} has no Cargo attachment", self.spec.product));
        };
        if token.trim().is_empty() {
            return Err("PLUMB_RELEASE_REGISTRY_TOKEN is required".into());
        }
        self.stamp(cargo, version)?;
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
        self.stamp(cargo, version)?;
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
            let checksum = crate::dispatch::release::record::digest(&archive)?.0;
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
            if crate::dispatch::release::record::digest(&archive)?.0 != checksum {
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

    fn ordered<'a>(&self, cargo: &'a Cargo) -> Result<&'a [String], String> {
        if cargo.packages.is_empty() {
            return Err("Cargo attachment must declare ordered packages".into());
        }
        Ok(&cargo.packages)
    }

    fn pins(cargo: &Cargo, identity: &Version) -> BTreeMap<String, String> {
        cargo
            .packages
            .iter()
            .map(|package| (package.clone(), identity.to_string()))
            .collect()
    }

    fn stamp(&self, cargo: &Cargo, version: &str) -> Result<(), String> {
        let identity = release(version)?;
        let pins = Self::pins(cargo, &identity);
        let workspace = Workspace::read(&self.spec.root)?;
        let root = self.spec.root.join("Cargo.toml");
        let mut document = manifest::read(&root)?;
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
        if document["workspace"]["package"]["version"]
            .as_str()
            .is_some()
        {
            document["workspace"]["package"]["version"] = toml_edit::value(identity.to_string());
        } else {
            document["package"]["version"] = toml_edit::value(identity.to_string());
        }
        manifest::write(&root, &document)?;
        for package in &cargo.packages {
            let (seat, _) = workspace.package(package)?;
            let mut document = manifest::read(seat)?;
            if document["package"]["version"].as_str().is_some() {
                document["package"]["version"] = toml_edit::value(identity.to_string());
            }
            manifest::dependencies(&mut document, &pins)?;
            manifest::write(seat, &document)?;
        }
        let mut document = manifest::read(&root)?;
        manifest::dependencies(&mut document, &pins)?;
        manifest::write(&root, &document)
    }

    fn command<const N: usize>(&self, args: [&str; N], token: &str) -> Result<(), String> {
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

    fn archive(&self, package: &str, version: &Version) -> PathBuf {
        self.spec
            .root
            .join("target/package")
            .join(format!("{package}-{version}.crate"))
    }
}

fn inspect(path: &Path, package: &str, version: &Version) -> Result<(), String> {
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
