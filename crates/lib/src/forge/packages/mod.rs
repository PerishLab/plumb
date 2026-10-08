mod cargo;
mod manifest;
mod npm;
mod process;
mod registry;

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub ecosystem: String,
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Resolution {
    pub context: String,
    pub tree: String,
    pub packages: Vec<Package>,
}

pub struct Plan {
    edits: Vec<manifest::Edit>,
    required: BTreeSet<(String, String)>,
    pub resolution: Resolution,
}

impl Plan {
    pub fn read(root: &Path, context: &str) -> Result<Self, String> {
        let paths = process::git(root, &["ls-files", "-z"])?;
        let mut edits = Vec::new();
        let mut names = BTreeSet::new();
        for path in paths.split('\0').filter(|path| !path.is_empty()) {
            if let Some(edit) = manifest::read(root, path, &mut names)? {
                edits.push(edit);
            }
        }
        let required = names.clone();
        let mut registered = BTreeSet::new();
        cargo::Lock(root).names(&mut registered)?;
        for name in names.iter().filter(|(family, _)| family == "cargo") {
            if !registered.contains(name) {
                return Err(format!(
                    "first-party Cargo dependency {} has no matching perish lock entry",
                    name.1
                ));
            }
        }
        names.extend(registered);
        npm::Lock(root).names(&mut names)?;
        if names.iter().any(|(family, _)| family == "cargo") && !root.join("Cargo.lock").is_file() {
            return Err("first-party Cargo resolution requires a committed lockfile".into());
        }
        if names.iter().any(|(family, _)| family == "npm") && !root.join("pnpm-lock.yaml").is_file()
        {
            return Err("first-party npm resolution requires a committed pnpm lockfile".into());
        }
        let mut packages = Vec::new();
        for (ecosystem, name) in names {
            packages.push(Package {
                version: registry::latest(root, &ecosystem, &name)?,
                ecosystem,
                name,
            });
        }
        Ok(Self {
            edits,
            required,
            resolution: Resolution {
                context: context.into(),
                tree: String::new(),
                packages,
            },
        })
    }

    pub fn apply(&self, root: &Path) -> Result<(), String> {
        let mut expanded = None;
        for _ in 0..4 {
            let plan = expanded.as_ref().unwrap_or(self);
            for edit in &plan.edits {
                edit.write(root)?;
            }
            cargo::Lock(root).update(&plan.resolution.packages)?;
            npm::Lock(root).update(
                &plan.resolution.packages,
                plan.edits.iter().any(|edit| edit.npm()),
            )?;
            let held = locked(root, &plan.resolution.context)?;
            if held.packages.iter().any(|package| {
                !plan.resolution.packages.iter().any(|wanted| {
                    wanted.ecosystem == package.ecosystem && wanted.name == package.name
                })
            }) {
                expanded = Some(Self::read(root, &self.resolution.context)?);
                continue;
            }
            return plan.verify(root);
        }
        Err("first-party dependency graph did not settle within four resolutions".into())
    }

    pub fn verify(&self, root: &Path) -> Result<(), String> {
        cargo::Lock(root).verify(&self.resolution.packages)?;
        npm::Lock(root).verify(&self.resolution.packages)?;
        let held = locked(root, &self.resolution.context)?;
        for (ecosystem, name) in &self.required {
            if !held
                .packages
                .iter()
                .any(|package| &package.ecosystem == ecosystem && &package.name == name)
            {
                return Err(format!(
                    "declared first-party {ecosystem} dependency {name} is missing from its lock"
                ));
            }
        }
        Ok(())
    }
}

pub fn allowed(path: &str) -> bool {
    matches!(
        path.rsplit('/').next(),
        Some("Cargo.toml" | "Cargo.lock" | "package.json" | "pnpm-lock.yaml")
    )
}

#[cfg(test)]
mod tests;

pub fn locked(root: &Path, context: &str) -> Result<Resolution, String> {
    let mut packages = cargo::Lock(root).locked()?;
    packages.extend(npm::Lock(root).locked()?);
    packages.sort();
    packages.dedup();
    let tree = process::git(root, &["write-tree"])?;
    Ok(Resolution {
        context: context.into(),
        tree: tree.trim().into(),
        packages,
    })
}
