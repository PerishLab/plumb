use super::{Package, process};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct Lock<'a>(pub(super) &'a Path);

impl Lock<'_> {
    pub(super) fn locked(&self) -> Result<Vec<Package>, String> {
        let path = self.0.join("pnpm-lock.yaml");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text)
            .map_err(|error| format!("invalid pnpm lock: {error}"))?;
        let version = value
            .get("lockfileVersion")
            .and_then(serde_yaml_ng::Value::as_str)
            .ok_or("pnpm lock has no version")?;
        if version != "9.0" {
            return Err(format!("unsupported pnpm lock format {version}"));
        }
        let mut packages = Vec::new();
        for key in value
            .get("packages")
            .and_then(serde_yaml_ng::Value::as_mapping)
            .into_iter()
            .flat_map(|table| table.keys())
        {
            let key = key.as_str().ok_or("pnpm package key is not text")?;
            if !key.starts_with("@perishlab/") {
                continue;
            }
            let (name, version) = key
                .rsplit_once('@')
                .ok_or("invalid first-party pnpm lock key")?;
            packages.push(Package {
                ecosystem: "npm".into(),
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

    pub(super) fn update(&self, packages: &[Package], changed: bool) -> Result<(), String> {
        let held = self.locked()?;
        if !changed
            && packages
                .iter()
                .filter(|package| package.ecosystem == "npm")
                .all(|wanted| held.iter().any(|package| package == wanted))
        {
            return Ok(());
        }
        let argv = [
            "pnpm",
            "update",
            "--recursive",
            "--workspace-root",
            "--lockfile-only",
            "--ignore-scripts",
            "--no-save",
            "@perishlab/*",
        ]
        .map(str::to_string);
        process::run(self.0, "pnpm", &argv)?;
        Ok(())
    }

    pub(super) fn verify(&self, packages: &[Package]) -> Result<(), String> {
        for held in self.locked()? {
            let wanted = packages
                .iter()
                .find(|package| package.ecosystem == "npm" && package.name == held.name)
                .ok_or_else(|| {
                    format!(
                        "new first-party npm dependency {} requires fresh resolution",
                        held.name
                    )
                })?;
            if held.version != wanted.version {
                return Err(format!(
                    "npm locked {} at {}, expected stable {}",
                    held.name, held.version, wanted.version
                ));
            }
        }
        Ok(())
    }
}
