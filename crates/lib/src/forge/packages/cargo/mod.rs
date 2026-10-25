mod graph;
use super::{Package, manifest, process};
pub use graph::compatible;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const SOURCE: &str = "sparse+https://cargo.perish.uk/";

type Moves = BTreeSet<(String, String)>;
type Pin = (std::path::PathBuf, String, String);

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

    pub(super) fn update(&self, packages: &[Package], manifests: &[String]) -> Result<(), String> {
        let moves = self.moves(packages)?;
        if moves.is_empty() {
            return Ok(());
        }
        let released = self
            .locked()?
            .into_iter()
            .map(|package| (package.name, package.version))
            .collect::<Moves>();
        let wanted = packages
            .iter()
            .filter(|package| package.ecosystem == "cargo")
            .filter(|package| released.iter().any(|(name, _)| name == &package.name))
            .map(|package| (package.name.clone(), package.version.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut held = Vec::new();
        for seat in manifests {
            let file = self.0.join(seat);
            let text = std::fs::read_to_string(&file).map_err(|error| error.to_string())?;
            let pinned = manifest::edit(&text, &mut manifest::pin(&wanted))?;
            if pinned != text {
                held.push((file, text, pinned));
            }
        }
        let path = self.0.join("Cargo.lock");
        let before = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let settled = self.settle(&before, &released, &held);
        if settled.is_err() {
            std::fs::write(&path, &before).map_err(|error| error.to_string())?;
        }
        settled
    }

    fn settle(&self, before: &str, moves: &Moves, held: &[Pin]) -> Result<(), String> {
        let path = self.0.join("Cargo.lock");
        std::fs::write(&path, graph::without(before, moves)?).map_err(|error| error.to_string())?;
        let resolved = self.resolve(held);
        for (file, text, _) in held {
            std::fs::write(file, text).map_err(|error| error.to_string())?;
        }
        resolved?;
        let argv = ["cargo", "metadata", "--format-version", "1", "--locked"].map(str::to_string);
        process::run(self.0, "cargo", &argv)
            .map_err(|error| format!("Cargo lock does not hold the restored manifests: {error}"))?;
        let after = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
        graph::forced(before, &after)
    }

    fn resolve(&self, held: &[Pin]) -> Result<(), String> {
        for (file, _, pinned) in held {
            std::fs::write(file, pinned).map_err(|error| error.to_string())?;
        }
        let argv = ["cargo", "metadata", "--format-version", "1"].map(str::to_string);
        process::run(self.0, "cargo", &argv).map(|_| ())
    }

    fn moves(&self, packages: &[Package]) -> Result<Moves, String> {
        let held = self.locked()?;
        let mut moves = BTreeSet::new();
        for wanted in packages
            .iter()
            .filter(|package| package.ecosystem == "cargo")
        {
            for package in held
                .iter()
                .filter(|package| package.name == wanted.name && package.version != wanted.version)
            {
                moves.insert((package.name.clone(), package.version.clone()));
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

#[cfg(test)]
mod tests;
