use super::{Package, process};
use std::collections::BTreeSet;
use std::path::Path;

const SOURCE: &str = "sparse+https://cargo.perish.uk/";

pub(super) struct Lock<'a>(pub(super) &'a Path);

impl Lock<'_> {
    pub(super) fn locked(&self) -> Result<Vec<Package>, String> {
        let path = self.0.join("Cargo.lock");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
        let value: toml::Value =
            toml::from_str(&text).map_err(|error| format!("invalid Cargo.lock: {error}"))?;
        let mut packages = Vec::new();
        for item in value
            .get("package")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            if item.get("source").and_then(toml::Value::as_str) != Some(SOURCE) {
                continue;
            }
            let name = item
                .get("name")
                .and_then(toml::Value::as_str)
                .ok_or("locked Cargo package has no name")?;
            let version = item
                .get("version")
                .and_then(toml::Value::as_str)
                .ok_or("locked Cargo package has no version")?;
            packages.push(Package {
                ecosystem: "cargo".into(),
                name: name.into(),
                version: version.into(),
            });
        }
        Ok(packages)
    }

    pub(super) fn names(&self, names: &mut BTreeSet<(String, String)>) -> Result<(), String> {
        for package in self.locked()? {
            names.insert((package.ecosystem, package.name));
        }
        Ok(())
    }

    pub(super) fn update(&self, packages: &[Package]) -> Result<(), String> {
        for wanted in packages
            .iter()
            .filter(|package| package.ecosystem == "cargo")
        {
            for held in self
                .locked()?
                .into_iter()
                .filter(|package| package.name == wanted.name && package.version != wanted.version)
            {
                let spec = format!("{SOURCE}#{}@{}", held.name, held.version);
                let argv = [
                    "cargo",
                    "update",
                    "--package",
                    &spec,
                    "--precise",
                    &wanted.version,
                ]
                .map(str::to_string);
                process::run(self.0, "cargo", &argv)?;
            }
        }
        Ok(())
    }

    pub(super) fn verify(&self, packages: &[Package]) -> Result<(), String> {
        for held in self.locked()? {
            let wanted = packages
                .iter()
                .find(|package| package.ecosystem == "cargo" && package.name == held.name)
                .ok_or_else(|| {
                    format!(
                        "new first-party Cargo dependency {} requires fresh resolution",
                        held.name
                    )
                })?;
            if held.version != wanted.version {
                return Err(format!(
                    "Cargo locked {} at {}, expected stable {}",
                    held.name, held.version, wanted.version
                ));
            }
        }
        Ok(())
    }
}
