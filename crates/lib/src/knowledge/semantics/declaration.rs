use super::wire::Wire;
use crate::semantics::Decision;
use crate::windtun::qualification::Fault;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Judgment {
    pub schema: String,
    pub product: String,
    pub before: String,
    pub after: String,
    pub anchor: String,
    pub decision: Decision,
    pub basis: String,
}

impl Judgment {
    pub const SCHEMA: &str = "plumb.release.review/v1";

    pub fn parse(body: &str) -> Result<Self, Fault> {
        let held: Self = Wire::marked(body, Self::SCHEMA)?;
        Wire::require(
            held.schema == Self::SCHEMA,
            "unsupported semantic review schema",
        )?;
        Wire::name(&held.product)?;
        Wire::hex(&held.before, 64)?;
        Wire::hex(&held.after, 64)?;
        Wire::hex(&held.anchor, 64)?;
        Wire::require(
            !held.basis.trim().is_empty() && held.basis.len() <= 16 * 1024,
            "semantic review requires a bounded actual basis",
        )?;
        Ok(held)
    }

    pub fn supplied(&self, reference: &str) -> crate::semantics::Review {
        crate::semantics::Review {
            before: self.before.clone(),
            after: self.after.clone(),
            anchor: self.anchor.clone(),
            decision: self.decision,
            reference: reference.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub schema: String,
    pub product: String,
    pub marker: String,
    pub snapshot: String,
    pub world: String,
    pub basis: String,
}

impl Definition {
    pub const SCHEMA: &str = "plumb.release.adoption/v1";

    pub fn parse(body: &str) -> Result<Self, Fault> {
        let held: Self = Wire::marked(body, Self::SCHEMA)?;
        Wire::require(
            held.schema == Self::SCHEMA,
            "unsupported baseline adoption review",
        )?;
        Wire::name(&held.product)?;
        super::super::version::Markers::stable(&held.marker).map_err(Fault::refused)?;
        Wire::hex(&held.snapshot, 64)?;
        Wire::hex(&held.world, 64)?;
        Wire::require(
            !held.basis.trim().is_empty() && held.basis.len() <= 16 * 1024,
            "baseline adoption requires its actual bounded basis",
        )?;
        Ok(held)
    }
}
