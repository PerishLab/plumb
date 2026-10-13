use super::{Package, Resolution, npm, process, registry};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub schema: u32,
    pub resolution: Resolution,
}

impl Observation {
    pub fn new(resolution: Resolution) -> Self {
        Self {
            schema: 1,
            resolution,
        }
    }

    pub fn npm(root: &Path, context: &str) -> Result<Self, String> {
        let mut packages = Vec::new();
        let names = npm::Lock(root)
            .locked()?
            .into_iter()
            .map(|held| held.name)
            .collect::<BTreeSet<_>>();
        for name in names {
            packages.push(Package {
                version: registry::latest(root, "npm", &name)?,
                ecosystem: "npm".into(),
                name,
            });
        }
        Ok(Self::new(Resolution {
            context: context.into(),
            tree: process::git(root, &["write-tree"])?.trim().into(),
            packages,
        }))
    }

    pub fn read(root: &Path, text: &str) -> Result<Self, String> {
        let held: Self =
            serde_json::from_str(text).map_err(|_| "invalid package observation".to_string())?;
        if held.schema != 1
            || !["local-locked", "ci-latest"].contains(&held.resolution.context.as_str())
        {
            return Err("package observation schema or context differs".into());
        }
        if held.resolution.tree != process::git(root, &["write-tree"])?.trim() {
            return Err("package observation tree differs".into());
        }
        let expected = npm::Lock(root)
            .locked()?
            .into_iter()
            .map(|held| held.name)
            .collect::<BTreeSet<_>>();
        let packages = held
            .resolution
            .packages
            .iter()
            .filter(|package| package.ecosystem == "npm")
            .collect::<Vec<_>>();
        let names = packages
            .iter()
            .map(|package| package.name.clone())
            .collect::<BTreeSet<_>>();
        if names != expected || names.len() != packages.len() {
            return Err("package observation npm set differs".into());
        }
        for package in packages {
            let version = semver::Version::parse(&package.version)
                .map_err(|_| "invalid package observation version")?;
            if !version.pre.is_empty() {
                return Err("package observation latest is a prerelease".into());
            }
        }
        Ok(held)
    }

    pub fn latest(&self, name: &str) -> Option<&str> {
        self.resolution
            .packages
            .iter()
            .find(|package| package.ecosystem == "npm" && package.name == name)
            .map(|package| package.version.as_str())
    }
}

pub fn latest(root: &Path, name: &str) -> Result<String, String> {
    if !name.starts_with("@perishlab/") || name.chars().any(|ch| ch.is_whitespace()) {
        return Err("npm observation requires a first-party package name".into());
    }
    registry::latest(root, "npm", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        process::git(root.path(), &["init", "-q"]).unwrap();
        std::fs::write(
            root.path().join("pnpm-lock.yaml"),
            "lockfileVersion: '9.0'\npackages:\n  '@perishlab/design@0.3.14': {}\n",
        )
        .unwrap();
        process::git(root.path(), &["add", "pnpm-lock.yaml"]).unwrap();
        root
    }

    fn observation(root: &Path) -> Observation {
        Observation::new(Resolution {
            context: "local-locked".into(),
            tree: process::git(root, &["write-tree"]).unwrap().trim().into(),
            packages: vec![Package {
                ecosystem: "npm".into(),
                name: "@perishlab/design".into(),
                version: "0.3.15".into(),
            }],
        })
    }

    #[test]
    fn bound() {
        let root = fixture();
        let held = observation(root.path());
        let read = Observation::read(root.path(), &serde_json::to_string(&held).unwrap()).unwrap();
        assert_eq!(read.latest("@perishlab/design"), Some("0.3.15"));
        assert!(Observation::read(root.path(), "malformed synthetic credential").is_err());
        for change in 0..6 {
            let mut held = observation(root.path());
            match change {
                0 => held.schema = 99,
                1 => held.resolution.context = "local-locked-as-latest".into(),
                2 => held.resolution.tree = "foreign".into(),
                3 => held.resolution.packages.clear(),
                4 => held
                    .resolution
                    .packages
                    .push(held.resolution.packages[0].clone()),
                _ => held.resolution.packages[0].version = "0.3.16-rc.1".into(),
            }
            assert!(
                Observation::read(root.path(), &serde_json::to_string(&held).unwrap()).is_err()
            );
        }
        std::fs::write(root.path().join("changed"), "another tree").unwrap();
        process::git(root.path(), &["add", "changed"]).unwrap();
        assert!(Observation::read(root.path(), &serde_json::to_string(&held).unwrap()).is_err());
    }
}
