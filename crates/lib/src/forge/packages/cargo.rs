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
        let moves = self.moves(packages)?;
        if moves.len() > 1 {
            let mut argv = vec!["cargo".to_string(), "update".to_string()];
            for (spec, _) in &moves {
                argv.push("--package".to_string());
                argv.push(spec.clone());
            }
            process::run(self.0, "cargo", &argv)?;
        }
        for (spec, version) in self.moves(packages)? {
            let argv =
                ["cargo", "update", "--package", &spec, "--precise", &version].map(str::to_string);
            process::run(self.0, "cargo", &argv)?;
        }
        Ok(())
    }

    fn moves(&self, packages: &[Package]) -> Result<Vec<(String, String)>, String> {
        let held = self.locked()?;
        let mut moves = Vec::new();
        for wanted in packages
            .iter()
            .filter(|package| package.ecosystem == "cargo")
        {
            for package in held
                .iter()
                .filter(|package| package.name == wanted.name && package.version != wanted.version)
            {
                moves.push((
                    format!("{SOURCE}#{}@{}", package.name, package.version),
                    wanted.version.clone(),
                ));
            }
        }
        Ok(moves)
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
