use super::{Spec, Target};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const LINUX: &str = "x86_64-unknown-linux-gnu";

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binary {
    targets: Option<Vec<String>>,
    install: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct Executable {
    pub name: String,
    pub targets: Vec<String>,
    pub install: bool,
}

impl Executable {
    pub fn carries(&self, triple: &str) -> bool {
        self.targets.iter().any(|held| held == triple)
    }
}

pub(super) fn resolve(
    binaries: &[String],
    union: &[String],
    tables: BTreeMap<String, Binary>,
) -> Result<Vec<Executable>, String> {
    if let Some(name) = tables.keys().find(|name| !binaries.contains(name)) {
        return Err(format!("[release.binary.{name}] names no declared binary"));
    }
    binaries
        .iter()
        .map(|name| {
            let table = tables.get(name).cloned().unwrap_or_default();
            Ok(Executable {
                targets: narrowed(name, union, table.targets)?,
                install: table.install.unwrap_or(true),
                name: name.clone(),
            })
        })
        .collect()
}

fn narrowed(
    name: &str,
    union: &[String],
    held: Option<Vec<String>>,
) -> Result<Vec<String>, String> {
    let Some(targets) = held else {
        return Ok(union.to_vec());
    };
    if targets.is_empty() {
        return Err(format!("binary {name} must declare at least one target"));
    }
    let mut seen = BTreeSet::new();
    for triple in &targets {
        if !union.contains(triple) {
            return Err(format!(
                "binary {name} target {triple} is not a release target"
            ));
        }
        if !seen.insert(triple) {
            return Err(format!("binary {name} declares target {triple} twice"));
        }
    }
    Ok(targets)
}

impl Spec {
    pub fn executable(&self, name: &str) -> Option<&Executable> {
        self.executables.iter().find(|held| held.name == name)
    }

    pub fn primary(&self) -> Option<&str> {
        self.binaries
            .iter()
            .find(|name| **name == self.product)
            .or_else(|| self.binaries.first())
            .map(String::as_str)
    }

    pub fn image(&self) -> Option<&str> {
        let oci = self.oci.as_ref()?;
        oci.binary.as_deref().or_else(|| self.primary())
    }

    pub fn lines(&self) -> Vec<String> {
        self.executables
            .iter()
            .map(|held| {
                let mut qualifiers = Vec::new();
                if held.targets.len() != self.target.len() {
                    qualifiers.push(format!("on {}", held.targets.join(" ")));
                }
                if !held.install {
                    qualifiers.push("uninstalled".to_string());
                }
                if qualifiers.is_empty() {
                    held.name.clone()
                } else {
                    format!("{} ({})", held.name, qualifiers.join(", "))
                }
            })
            .collect()
    }

    pub fn placements(&self) -> Vec<String> {
        let deb = self.deb.as_ref().map(|held| format!("deb {}", held.binary));
        let oci = self.image().map(|held| format!("oci {held}"));
        deb.into_iter().chain(oci).collect()
    }

    pub(super) fn server(&self, subject: &str, name: &str) -> Result<(), String> {
        let Some(executable) = self.executable(name) else {
            return Err(format!("{subject} names undeclared binary {name}"));
        };
        if !executable.carries(LINUX) {
            return Err(format!("{subject} binary {name} does not target {LINUX}"));
        }
        Ok(())
    }

    pub(super) fn carried(&self, target: &Target) -> Result<(), String> {
        if self
            .executables
            .iter()
            .any(|held| held.carries(&target.triple))
        {
            Ok(())
        } else {
            Err(format!("no binary carries target {}", target.triple))
        }
    }
}
