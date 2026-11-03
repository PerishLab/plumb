use super::manifest::Manifest;
use super::model::{Artifact, Metadata, Package, Plan, Program, Target, contained};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

impl Plan {
    pub fn read(root: &Path, metadata: &str, stream: &str) -> Result<Self, String> {
        let root = root.canonicalize().map_err(|error| error.to_string())?;
        let metadata: Metadata = serde_json::from_str(metadata)
            .map_err(|error| format!("invalid Cargo metadata: {error}"))?;
        metadata.validate(&root)?;
        let artifacts = super::messages::read(stream)?;
        let mut plan = Self {
            programs: Vec::new(),
            docs: false,
            fallback: Vec::new(),
        };
        for package in metadata.selected() {
            plan.package(package, &artifacts, &metadata)?;
        }
        for artifact in artifacts.iter().filter(|artifact| artifact.profile.test) {
            if artifact.executable.is_some() && !metadata.defaults.contains(&artifact.package) {
                return Err("Cargo reported a test outside its default package selection".into());
            }
        }
        plan.programs.sort_by(|left, right| {
            right.target.unit().cmp(&left.target.unit()).then_with(|| {
                (&left.package, &left.target.name, &left.target.kind).cmp(&(
                    &right.package,
                    &right.target.name,
                    &right.target.kind,
                ))
            })
        });
        plan.fallback.sort();
        plan.fallback.dedup();
        Ok(plan)
    }

    fn package(
        &mut self,
        package: &Package,
        artifacts: &[Artifact],
        metadata: &Metadata,
    ) -> Result<(), String> {
        let manifest = Manifest::read(package)?;
        let held = artifacts
            .iter()
            .filter(|artifact| artifact.package == package.id)
            .collect::<Vec<_>>();
        let features = held
            .iter()
            .map(|artifact| &artifact.features)
            .collect::<BTreeSet<_>>();
        let mut seen = BTreeSet::new();
        for artifact in held.iter().filter(|artifact| artifact.profile.test) {
            let Some(executable) = &artifact.executable else {
                continue;
            };
            let target = package.target(&artifact.target)?;
            if !target.test || !target.required.is_subset(&artifact.features) {
                return Err(format!(
                    "Cargo test selection disagrees for {}",
                    target.name
                ));
            }
            if !seen.insert((target.name.clone(), target.kind.clone())) {
                return Err(format!("duplicate Cargo test artifact for {}", target.name));
            }
            let custom = match manifest.custom(package, target) {
                Ok(custom) => custom,
                Err(reason) => {
                    self.fallback.push(format!("{}: {reason}", package.name));
                    continue;
                }
            };
            if custom {
                self.fallback
                    .push(format!("{}: custom harness {}", package.name, target.name));
            }
            self.programs.push(Program {
                package: package.id.clone(),
                name: package.name.clone(),
                target: target.clone(),
                executable: contained(
                    executable,
                    &metadata
                        .output
                        .canonicalize()
                        .map_err(|error| error.to_string())?,
                )?,
                directory: package
                    .manifest
                    .parent()
                    .ok_or("package has no directory")?
                    .to_path_buf(),
                features: artifact.features.clone(),
                custom,
            });
        }
        for target in &package.targets {
            self.docs |= target.library() && target.doctest;
            self.expected(package, target, &features, &seen);
        }
        Ok(())
    }

    fn expected(
        &mut self,
        package: &Package,
        target: &Target,
        features: &BTreeSet<&BTreeSet<String>>,
        seen: &BTreeSet<(String, Vec<String>)>,
    ) {
        if !target.test {
            return;
        }
        if target.role().is_err() {
            self.fallback.push(format!(
                "{}: unsupported test target {}",
                package.name, target.name
            ));
            return;
        }
        if seen.contains(&(target.name.clone(), target.kind.clone())) {
            return;
        }
        if !target.required.is_empty() && features.len() == 1 {
            let active = features.iter().next().expect("one feature context");
            if !target.required.is_subset(active) {
                return;
            }
        }
        self.fallback.push(format!(
            "{}: selected target {} has no qualified artifact",
            package.name, target.name
        ));
    }
}

impl Metadata {
    fn selected(&self) -> impl Iterator<Item = &Package> {
        self.packages
            .iter()
            .filter(|package| self.defaults.contains(&package.id))
    }

    fn validate(&self, root: &Path) -> Result<(), String> {
        if contained(&self.root, root)? != root {
            return Err("Cargo workspace root differs from the guarded root".into());
        }
        if self.defaults.is_empty() || !self.defaults.is_subset(&self.members) {
            return Err("Cargo metadata has no complete default package selection".into());
        }
        let mut packages = BTreeMap::new();
        for package in &self.packages {
            if packages.insert(&package.id, package).is_some() {
                return Err("Cargo metadata repeats a package identity".into());
            }
        }
        if self.members.iter().any(|id| !packages.contains_key(id)) {
            return Err("Cargo metadata omits a workspace package".into());
        }
        for package in self.selected() {
            contained(&package.manifest, root)?;
            let mut targets = BTreeSet::new();
            for target in &package.targets {
                contained(&target.source, root)?;
                if !targets.insert((&target.name, &target.kind)) {
                    return Err("Cargo metadata repeats a target identity".into());
                }
            }
        }
        Ok(())
    }
}

impl Package {
    fn target(&self, reported: &Target) -> Result<&Target, String> {
        self.targets
            .iter()
            .find(|target| target.agrees(reported))
            .ok_or_else(|| {
                format!(
                    "Cargo artifact does not match a declared target in {}",
                    self.name
                )
            })
    }
}
