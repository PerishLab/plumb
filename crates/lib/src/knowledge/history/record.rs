use super::super::{Native, Reference, Tag, wire::Wire};
use crate::semantics::version::Markers;
use crate::windtun::qualification::Fault;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub schema: String,
    pub product: String,
    pub repository: Value,
    pub root: Reference,
    pub previous: Option<Reference>,
    pub sequence: u64,
    pub markers: Vec<Tag>,
    pub native: Native,
}

impl Checkpoint {
    pub const SCHEMA: &str = "wharf.release.semantics.history/v1";

    pub fn parse(raw: &[u8]) -> Result<Self, Fault> {
        let held: Self = Wire::parse(raw)?;
        Wire::require(
            Wire::canonical(&held)? == raw,
            "history checkpoint is not canonical",
        )?;
        held.validate()?;
        Ok(held)
    }

    pub fn encode(&self) -> Result<Vec<u8>, Fault> {
        self.validate()?;
        let raw = Wire::canonical(self)?;
        if raw.len() > Wire::LIMIT {
            return Err(Fault::unknown(
                "history checkpoint exceeds its observation bound",
            ));
        }
        Ok(raw)
    }

    pub fn validate(&self) -> Result<(), Fault> {
        Wire::require(
            self.schema == Self::SCHEMA,
            "unsupported history checkpoint",
        )?;
        Wire::name(&self.product)?;
        self.root.validate(&self.product)?;
        self.native.validate()?;
        Wire::require(
            self.sequence > 0 && (self.sequence == 1) == self.previous.is_none(),
            "history checkpoint has no exact predecessor",
        )?;
        if let Some(reference) = &self.previous {
            reference.validate(&self.product)?;
        }
        Wire::require(
            self.repository["full_name"] == format!("PerishLab/{}", self.product)
                && self.repository["id"].as_u64().is_some_and(|id| id > 0),
            "history repository identity differs",
        )?;
        Wire::text(&self.repository["node_id"])?;
        Wire::require(
            !self.markers.is_empty() && self.markers.len() <= 4096,
            "history reservations are absent or unbounded",
        )?;
        for pair in self.markers.windows(2) {
            Wire::require(
                pair[0].marker < pair[1].marker,
                "history reservations repeat or are unordered",
            )?;
        }
        for tag in &self.markers {
            Markers::parse(&tag.marker).map_err(Fault::refused)?;
            Wire::hex(&tag.commit, 40)?;
        }
        Ok(())
    }

    pub fn follows(&self, reference: &Reference, previous: &Self) -> Result<(), Fault> {
        self.validate()?;
        previous.validate()?;
        let raw = previous.encode()?;
        Wire::require(
            self.previous.as_ref() == Some(reference)
                && reference.sha256 == Wire::sha(&raw)
                && reference.size == raw.len() as u64,
            "history predecessor reference differs from its exact checkpoint",
        )?;
        Wire::require(
            previous.sequence.checked_add(1) == Some(self.sequence),
            "history sequence did not advance exactly once",
        )?;
        Wire::require(
            self.product == previous.product
                && self.repository == previous.repository
                && self.root == previous.root,
            "history sequence, product, repository or original root changed",
        )?;
        let current: BTreeMap<_, _> = self
            .markers
            .iter()
            .map(|tag| (&tag.marker, &tag.commit))
            .collect();
        Wire::require(
            previous
                .markers
                .iter()
                .all(|tag| current.get(&tag.marker) == Some(&&tag.commit)),
            "history erased or retargeted an earlier reservation",
        )
    }
}
