use super::{identity, rejoin};
use crate::shape::release::Spec;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Default)]
pub struct Release {
    pub attachments: BTreeSet<String>,
    pub widths: BTreeMap<String, usize>,
    pub refusal: Option<String>,
    pub blind: Option<String>,
    pub unsettled: Option<String>,
}

pub struct Root<'a>(pub &'a Path);

impl Root<'_> {
    pub fn release(&self) -> Release {
        let manifest = self.0.join("plumb.toml");
        if !manifest.is_file() || !seated(&manifest) {
            return Release::default();
        }
        match Spec::read(&manifest) {
            Ok(spec) => self.held(&spec),
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

    fn held(&self, spec: &Spec) -> Release {
        let attachments = attachments(spec);
        Release {
            attachments,
            widths: widths(spec),
            refusal: spec.ship().err(),
            blind: identity::Seat(self.0).blind(&spec.product, plumb::commit!("PLUMB")),
            unsettled: rejoin::unsettled(self.0),
        }
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
    if spec.skill {
        found.insert("skill".to_string());
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
