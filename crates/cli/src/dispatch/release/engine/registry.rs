use super::super::model::{Cargo, Spec};
use super::ledger;
use super::workspace::{Workspace, release};
use flate2::read::GzDecoder;
use semver::Version;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml_edit::{DocumentMut, Item, Value};

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
        for (index, package) in cargo.packages.iter().enumerate() {
            if index == 0 {
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
            } else {
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
            }
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
        for package in &cargo.packages {
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
            let checksum = super::super::record::digest(&archive)?.0;
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
            if super::super::record::digest(&archive)?.0 != checksum {
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

    fn stamp(&self, cargo: &Cargo, version: &str) -> Result<(), String> {
        let identity = release(version)?;
        let workspace = Workspace::read(&self.spec.root)?;
        let root = self.spec.root.join("Cargo.toml");
        let mut document = read(&root)?;
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
        write(&root, &document)?;
        for package in &cargo.packages {
            let (manifest, _) = workspace.package(package)?;
            let mut document = read(manifest)?;
            if document["package"]["version"].as_str().is_some() {
                document["package"]["version"] = toml_edit::value(identity.to_string());
            }
            dependencies(&mut document, &cargo.packages, &identity.to_string())?;
            write(manifest, &document)?;
        }
        let mut document = read(&root)?;
        dependencies(&mut document, &cargo.packages, &identity.to_string())?;
        write(&root, &document)
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

fn dependencies(
    document: &mut DocumentMut,
    packages: &[String],
    version: &str,
) -> Result<(), String> {
    for section in ["dependencies", "build-dependencies", "dev-dependencies"] {
        let Some(table) = document.get_mut(section).and_then(Item::as_table_mut) else {
            continue;
        };
        for package in packages {
            let Some(item) = table.get_mut(package) else {
                continue;
            };
            dependency(item, version);
        }
    }
    Ok(())
}

fn dependency(item: &mut Item, version: &str) {
    if let Some(detail) = item.as_inline_table_mut() {
        if detail.contains_key("path") {
            detail.insert("version", Value::from(format!("={version}")));
        }
    } else if let Some(detail) = item.as_table_mut()
        && detail.contains_key("path")
    {
        detail["version"] = toml_edit::value(format!("={version}"));
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

fn read(path: &Path) -> Result<DocumentMut, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?
        .parse()
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))
}

fn write(path: &Path, document: &DocumentMut) -> Result<(), String> {
    std::fs::write(path, document.to_string())
        .map_err(|error| format!("cannot stamp {}: {error}", path.display()))
}
