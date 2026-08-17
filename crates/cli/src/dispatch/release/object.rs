use super::model::Spec;
use super::record::Input;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const CHART: &str = "chart";
pub const CFWORKER: &str = "cfworker";

pub fn cargo(package: &str) -> String {
    format!("cargo/{package}")
}

pub fn npm(package: &str) -> String {
    format!("npm/{package}")
}

pub struct Seat<'a>(pub &'a Spec);

pub struct Object {
    pub name: String,
    pub roots: Vec<PathBuf>,
}

pub struct Held(BTreeMap<String, String>);

impl Held {
    pub fn since(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }
}

impl Seat<'_> {
    pub fn declared(&self) -> Result<Vec<Object>, String> {
        let spec = self.0;
        let mut held: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
        if let Some(modules) = &spec.npm {
            for package in &modules.packages {
                let bare = package.rsplit('/').next().unwrap_or_default();
                held.insert(npm(bare), vec![PathBuf::from("packages").join(bare)]);
            }
        }
        if let Some(chart) = &spec.chart {
            let name = chart.chart.rsplit('/').next().unwrap_or_default();
            held.insert(CHART.to_string(), vec![PathBuf::from("charts").join(name)]);
        }
        if spec.cfworker.is_some() {
            held.insert(CFWORKER.to_string(), vec![PathBuf::from("apps")]);
        }
        self.crates(&mut held)?;
        for (name, extra) in &spec.depends {
            let roots = held
                .get_mut(name)
                .ok_or_else(|| format!("{name} names no declared ship object"))?;
            for path in extra {
                if !spec.root.join(path).exists() {
                    return Err(format!("declared root is absent: {}", path.display()));
                }
                roots.push(path.clone());
            }
        }
        Ok(held
            .into_iter()
            .map(|(name, mut roots)| {
                roots.sort();
                roots.dedup();
                Object { name, roots }
            })
            .collect())
    }

    pub fn inputs(
        &self,
        toolchain: &str,
        version: &str,
    ) -> Result<BTreeMap<String, Input>, String> {
        let prior = self.baseline()?;
        self.measure(toolchain, version, &prior)
    }

    pub fn held(&self, toolchain: &str, version: &str) -> Held {
        let prior = self.baseline().unwrap_or_else(|error| {
            println!("  cannot read the published baseline, projecting everything: {error}");
            BTreeMap::new()
        });
        let measured = self.measure(toolchain, version, &prior).unwrap_or_default();
        Held(
            measured
                .into_iter()
                .filter(|(_, input)| input.since != version)
                .map(|(name, input)| (name, input.since))
                .collect(),
        )
    }

    fn baseline(&self) -> Result<BTreeMap<String, Input>, String> {
        let spec = self.0;
        if spec.authority.is_empty() {
            return Ok(BTreeMap::new());
        }
        let url = format!(
            "{}/v1/channels/stable.json",
            spec.authority.trim_end_matches('/')
        );
        let Some(pointer) = super::verify::optional(&url)? else {
            return Ok(BTreeMap::new());
        };
        let seat = pointer
            .pointer("/seal/url")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "the stable pointer names no seal".to_string())?;
        let Some(seal) = super::verify::optional(seat)? else {
            return Ok(BTreeMap::new());
        };
        let Some(held) = seal.get("inputs").filter(|held| !held.is_null()) else {
            return Ok(BTreeMap::new());
        };
        serde_json::from_value(held.clone())
            .map_err(|error| format!("cannot read published inputs from {seat}: {error}"))
    }

    fn measure(
        &self,
        toolchain: &str,
        version: &str,
        prior: &BTreeMap<String, Input>,
    ) -> Result<BTreeMap<String, Input>, String> {
        let snapshot = plumb::snapshot::Snapshot::read(self.0.root.as_path())
            .map_err(|error| format!("cannot read the tracked tree: {error}"))?;
        let mut held = BTreeMap::new();
        for object in self.declared()? {
            let mut digest = Sha256::new();
            digest.update(toolchain.as_bytes());
            digest.update([0]);
            let mut seen = 0usize;
            for entry in snapshot.entries() {
                if !object.roots.iter().any(|root| under(entry.path(), root)) {
                    continue;
                }
                seen += 1;
                digest.update(entry.path().as_bytes());
                digest.update([0]);
                digest.update(entry.bytes());
                digest.update([0]);
            }
            if seen == 0 {
                return Err(format!("{} covers no tracked leaf", object.name));
            }
            let hash = format!("{:x}", digest.finalize());
            let since = prior
                .get(&object.name)
                .filter(|seen| seen.hash == hash)
                .map_or(version, |seen| seen.since.as_str())
                .to_string();
            held.insert(object.name, Input { hash, since });
        }
        Ok(held)
    }

    fn crates(&self, held: &mut BTreeMap<String, Vec<PathBuf>>) -> Result<(), String> {
        let Some(attachment) = &self.0.cargo else {
            return Ok(());
        };
        let seats = super::engine::workspace::Workspace::read(&self.0.root)?.seats(&self.0.root);
        for package in &attachment.packages {
            let seat = seats
                .get(package)
                .ok_or_else(|| format!("cargo package {package} is not one workspace member"))?;
            held.insert(cargo(package), vec![seat.clone()]);
        }
        Ok(())
    }
}

fn under(path: &str, root: &Path) -> bool {
    let root = root.to_string_lossy();
    path == root || path.starts_with(&format!("{root}/"))
}
