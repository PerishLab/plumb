use super::identity;
use crate::shape::release::Spec;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Default)]
pub struct Release {
    pub attachments: BTreeSet<String>,
    pub callers: BTreeSet<String>,
    pub sources: BTreeSet<String>,
    pub widths: BTreeMap<String, usize>,
    pub refusal: Option<String>,
    pub blind: Option<String>,
}

pub struct Root<'a>(pub &'a Path);

impl Root<'_> {
    pub fn release(&self, lanes: &BTreeSet<String>) -> Release {
        let manifest = self.0.join("plumb.toml");
        if !manifest.is_file() || !seated(&manifest) {
            return Release::default();
        }
        match Spec::read(&manifest) {
            Ok(spec) => self.held(&spec, lanes),
            Err(refusal) => Release {
                refusal: Some(refusal),
                ..Release::default()
            },
        }
    }

    pub fn sites(&self) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        let Ok(entries) = std::fs::read_dir(self.0.join("apps")) else {
            return found;
        };
        for entry in entries.flatten() {
            if entry.path().join("wrangler.jsonc").exists() {
                found.insert(entry.file_name().to_string_lossy().to_string());
            }
        }
        found
    }

    fn held(&self, spec: &Spec, lanes: &BTreeSet<String>) -> Release {
        let attachments = attachments(spec);
        let mut callers = if attachments.is_empty() {
            BTreeSet::new()
        } else {
            self.callers()
        };
        if lanes.contains("ship") {
            callers.insert("ship".to_string());
        }
        let sources = self.sources(lanes, &attachments);
        Release {
            attachments,
            callers,
            sources,
            widths: widths(spec),
            refusal: None,
            blind: identity::Seat(self.0).blind(&spec.product, plumb::commit!("PLUMB")),
        }
    }

    fn callers(&self) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        let Ok(entries) = std::fs::read_dir(self.0.join(".forgejo/workflows")) else {
            return found;
        };
        for entry in entries.flatten() {
            let Ok(text) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            for line in text.lines() {
                let Some((_, called)) = line.split_once("/.forgejo/workflows/") else {
                    continue;
                };
                let name = called
                    .split(['@', ' ', '\t'])
                    .next()
                    .unwrap_or_default()
                    .trim_end_matches(".yml");
                if !name.is_empty() {
                    found.insert(name.to_string());
                }
            }
        }
        found
    }

    fn sources(
        &self,
        lanes: &BTreeSet<String>,
        attachments: &BTreeSet<String>,
    ) -> BTreeSet<String> {
        let unified = lanes.contains("ship");
        let internal = lanes.contains("exact.release") && lanes.contains("stable.release");
        if !attachments.contains("binary") || unified || internal {
            return BTreeSet::new();
        }
        ["release-exact", "release-stable"]
            .into_iter()
            .filter(|lane| lanes.contains(*lane) && self.bound(lane))
            .map(str::to_string)
            .collect()
    }

    fn bound(&self, lane: &str) -> bool {
        std::fs::read_to_string(
            self.0
                .join(".forgejo/workflows")
                .join(format!("{lane}.yml")),
        )
        .map(|text| {
            let loose = [
                "source_ref:",
                "source_commit:",
                "${{ inputs.ref }}",
                "\n      ref:\n",
            ];
            !loose.iter().any(|value| text.contains(value))
        })
        .unwrap_or(false)
    }
}

fn seated(manifest: &Path) -> bool {
    std::fs::read_to_string(manifest)
        .ok()
        .map(|text| match text.parse::<toml::Table>() {
            Ok(table) => table.contains_key("release"),
            Err(_) => true,
        })
        .unwrap_or(false)
}

fn attachments(spec: &Spec) -> BTreeSet<String> {
    let mut found: BTreeSet<String> = spec.surface().into_iter().map(str::to_string).collect();
    for (present, name) in [(spec.skill, "skill"), (spec.deb.is_some(), "deb")] {
        if present {
            found.insert(name.to_string());
        }
    }
    found
}

fn widths(spec: &Spec) -> BTreeMap<String, usize> {
    let mut found = BTreeMap::new();
    if let Some(cargo) = &spec.cargo {
        found.insert("cargo".to_string(), cargo.packages.len());
    }
    if let Some(npm) = &spec.npm {
        found.insert("npm".to_string(), npm.packages.len());
    }
    found
}
