use super::wire::Wire;
use crate::windtun::qualification::Fault;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
pub(super) struct Index {
    format: u32,
    pub cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Case {
    pub id: String,
    pub manifest: String,
    pub group: String,
}

impl Index {
    pub fn parse(raw: &[u8], paths: &[String]) -> Result<Self, Fault> {
        let text =
            std::str::from_utf8(raw).map_err(|_| Fault::refused("bundle index is not UTF-8"))?;
        let held: Self =
            toml::from_str(text).map_err(|_| Fault::refused("invalid bundle index projection"))?;
        Wire::require(held.format == 2, "unsupported bundle index format")?;
        Wire::require(
            (1..=128).contains(&held.cases.len()),
            "bundle case inventory exceeds bounds",
        )?;
        let mut ids = BTreeSet::new();
        let mut manifests = BTreeSet::new();
        for case in &held.cases {
            Wire::name(&case.id)?;
            Wire::name(&case.group)?;
            Wire::path(&case.manifest)?;
            Wire::require(
                ids.insert(&case.id) && manifests.insert(&case.manifest),
                "bundle repeats a case identity",
            )?;
            Wire::require(
                paths.contains(&case.manifest),
                "indexed case manifest is absent",
            )?;
        }
        Ok(held)
    }
}
