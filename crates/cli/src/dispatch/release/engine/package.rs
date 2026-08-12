use super::super::model::{Format, Spec};
use super::super::{artifact, artifact::Asset};
use super::{archive, debian, skill};
use crate::dispatch::release::engine::workspace::Workspace;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub struct Product<'a> {
    spec: &'a Spec,
}

pub struct Build<'a> {
    pub target: &'a str,
    pub version: &'a str,
    pub channel: &'a str,
    pub commit: &'a str,
    pub artifacts: &'a Path,
}

pub fn product(spec: &Spec) -> Product<'_> {
    Product { spec }
}

impl Product<'_> {
    pub fn matrix(&self) -> Result<String, String> {
        let include = self
            .spec
            .target
            .iter()
            .map(|target| {
                json!({
                    "runner": target.runner,
                    "target": target.triple,
                    "archive": target.archive,
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_string(&json!({ "include": include })).map_err(|error| error.to_string())
    }

    pub fn build(&self, input: Build<'_>) -> Result<String, String> {
        super::super::manager::intent(input.channel, input.version)?;
        super::super::proof::commit(input.commit)?;
        let target = self.spec.target(input.target)?;
        std::fs::create_dir_all(input.artifacts)
            .map_err(|error| format!("cannot create {}: {error}", input.artifacts.display()))?;
        let workspace = Workspace::read(&self.spec.root)?;
        let binaries = workspace.build(
            self.spec,
            super::workspace::Build {
                triple: &target.triple,
                version: input.version,
                channel: input.channel,
                commit: input.commit,
            },
        )?;
        let archive = input.artifacts.join(&target.archive);
        if archive.exists() {
            return Err(format!(
                "release artifact already exists: {}",
                archive.display()
            ));
        }
        archive::write(target.format, &archive, &binaries)?;
        self.inspect(target.format, &archive)?;
        if self.spec.deb.is_some() && target.triple == "x86_64-unknown-linux-gnu" {
            let path = input.artifacts.join(format!(
                "{}-x86_64-unknown-linux-gnu.deb",
                self.spec.product
            ));
            if path.exists() {
                return Err(format!(
                    "release artifact already exists: {}",
                    path.display()
                ));
            }
            debian::build(self.spec, input.version, &binaries, &path)?;
        }
        Ok(format!("built {} {}", self.spec.product, target.triple))
    }

    pub fn assemble(&self, version: &str, artifacts: &Path) -> Result<String, String> {
        let assets = artifact::list(self.spec, version)?;
        self.gather(artifacts, &assets)?;
        if self.spec.skill {
            let path = artifacts.join(format!("{}-skill.tar.gz", self.spec.product));
            if path.exists() {
                return Err(format!(
                    "release artifact already exists: {}",
                    path.display()
                ));
            }
            skill::build(self.spec, version, &path)?;
        }
        for asset in assets.iter().filter(|asset| asset.source.is_some()) {
            let source = asset.source.as_ref().expect("source should exist");
            let target = artifacts.join(&asset.file);
            if target.exists() {
                return Err(format!(
                    "release artifact already exists: {}",
                    target.display()
                ));
            }
            std::fs::copy(source, &target).map_err(|error| {
                format!(
                    "cannot stage {} as {}: {error}",
                    source.display(),
                    target.display()
                )
            })?;
        }
        self.verify(artifacts, &assets)?;
        Ok(format!("assembled {} {}", self.spec.product, version))
    }

    fn verify(&self, artifacts: &Path, assets: &[Asset]) -> Result<(), String> {
        let declared = self
            .spec
            .target
            .iter()
            .map(|target| target.archive.clone())
            .chain(assets.iter().map(|asset| asset.file.clone()))
            .collect::<BTreeSet<_>>();
        let found = std::fs::read_dir(artifacts)
            .map_err(|error| format!("cannot read {}: {error}", artifacts.display()))?
            .map(|entry| {
                let entry = entry.map_err(|error| error.to_string())?;
                if !entry.path().is_file() {
                    return Err(format!(
                        "artifact root contains non-file {}",
                        entry.path().display()
                    ));
                }
                Ok(entry.file_name().to_string_lossy().to_string())
            })
            .collect::<Result<BTreeSet<_>, String>>()?;
        if found != declared {
            return Err(format!(
                "artifact set disagrees: expected {declared:?}, found {found:?}"
            ));
        }
        for target in &self.spec.target {
            self.inspect(target.format, &artifacts.join(&target.archive))?;
        }
        if self.spec.skill {
            skill::verify(
                self.spec,
                &artifacts.join(format!("{}-skill.tar.gz", self.spec.product)),
            )?;
        }
        if self.spec.deb.is_some() {
            debian::verify(
                self.spec,
                &artifacts.join(format!(
                    "{}-x86_64-unknown-linux-gnu.deb",
                    self.spec.product
                )),
            )?;
        }
        Ok(())
    }

    fn inspect(&self, format: Format, path: &Path) -> Result<(), String> {
        let names = archive::names(format, path)?;
        for binary in &self.spec.binaries {
            let expected = if format == Format::Zip {
                format!("{binary}.exe")
            } else {
                binary.clone()
            };
            if !names.iter().any(|name| name == &expected) {
                return Err(format!("{} misses {expected}", path.display()));
            }
        }
        Ok(())
    }

    fn gather(&self, root: &Path, assets: &[Asset]) -> Result<(), String> {
        let expected = self
            .spec
            .target
            .iter()
            .map(|target| target.archive.clone())
            .chain(
                assets
                    .iter()
                    .filter(|asset| asset.key != "skill" && asset.source.is_none())
                    .map(|asset| asset.file.clone()),
            )
            .collect::<BTreeSet<_>>();
        let mut found = BTreeMap::<String, Vec<PathBuf>>::new();
        visit(root, &mut found)?;
        for name in expected {
            let paths = found.remove(&name).unwrap_or_default();
            if paths.len() != 1 {
                return Err(format!(
                    "artifact {name} has {} gathered copies",
                    paths.len()
                ));
            }
            let target = root.join(&name);
            if paths[0] != target {
                std::fs::rename(&paths[0], &target).map_err(|error| {
                    format!(
                        "cannot gather {} as {}: {error}",
                        paths[0].display(),
                        target.display()
                    )
                })?;
            }
        }
        if !found.is_empty() {
            return Err(format!(
                "artifact download contains undeclared files: {:?}",
                found.keys().collect::<Vec<_>>()
            ));
        }
        sweep(root)
    }
}

fn visit(path: &Path, found: &mut BTreeMap<String, Vec<PathBuf>>) -> Result<(), String> {
    for entry in std::fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_symlink() {
            return Err(format!(
                "artifact download refuses symbolic link {}",
                entry.path().display()
            ));
        }
        if kind.is_dir() {
            visit(&entry.path(), found)?;
        } else if kind.is_file() {
            found
                .entry(entry.file_name().to_string_lossy().to_string())
                .or_default()
                .push(entry.path());
        }
    }
    Ok(())
}

fn sweep(root: &Path) -> Result<(), String> {
    for entry in std::fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            std::fs::remove_dir_all(entry.path()).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}
