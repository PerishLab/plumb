use super::{Fault, Native, Read, Reference, Wire};
use crate::semantics::version::Markers;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub schema: String,
    pub product: String,
    pub marker: String,
    pub root: Reference,
    pub family: Reference,
    pub history: Reference,
    pub previous: Option<Reference>,
    pub sequence: u64,
    pub native: Native,
}

impl Selection {
    pub const SCHEMA: &str = "wharf.release.semantics.family.selection/v1";

    pub fn parse(raw: &[u8]) -> Result<Self, Fault> {
        let held: Self = Wire::parse(raw)?;
        Wire::require(held.encode()? == raw, "selection is not canonical")?;
        Ok(held)
    }

    pub fn encode(&self) -> Result<Vec<u8>, Fault> {
        Wire::require(self.schema == Self::SCHEMA, "unsupported family selection")?;
        Wire::name(&self.product)?;
        Markers::stable(&self.marker).map_err(Fault::refused)?;
        self.native.validate()?;
        for reference in [&self.root, &self.family, &self.history] {
            reference.validate(&self.product)?;
        }
        Wire::require(
            self.sequence > 0 && (self.sequence == 1) == self.previous.is_none(),
            "selection sequence has no exact predecessor",
        )?;
        if let Some(reference) = &self.previous {
            reference.validate(&self.product)?;
        }
        Wire::canonical(self)
    }

    pub fn reference(&self) -> Result<Reference, Fault> {
        let raw = self.encode()?;
        let sha256 = Wire::sha(&raw);
        Ok(Reference {
            url: format!(
                "https://releases.{}.perish.uk/v1/semantics/objects/{sha256}.json",
                self.product
            ),
            sha256,
            size: raw.len() as u64,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Selector {
    pub schema: String,
    pub reference: Reference,
}

impl Selector {
    pub const SCHEMA: &str = "wharf.release.semantics.family.head/v1";

    pub fn url(product: &str) -> Result<String, Fault> {
        Wire::name(product)?;
        Ok(format!(
            "https://releases.{product}.perish.uk/v1/semantics/family/latest.json"
        ))
    }

    pub fn encode(&self, product: &str) -> Result<Vec<u8>, Fault> {
        Wire::require(self.schema == Self::SCHEMA, "unsupported family selector")?;
        self.reference.validate(product)?;
        Wire::canonical(self)
    }

    pub fn inception(product: &str) -> Result<String, Fault> {
        Wire::name(product)?;
        Ok(format!(
            "https://releases.{product}.perish.uk/v1/semantics/family/initial.json"
        ))
    }

    pub(super) fn read(reader: &impl Read, product: &str) -> Result<Option<Self>, Fault> {
        Self::at(reader, &Self::url(product)?, product)
    }

    pub(super) fn first(reader: &impl Read, product: &str) -> Result<Option<Self>, Fault> {
        Self::at(reader, &Self::inception(product)?, product)
    }

    fn at(reader: &impl Read, url: &str, product: &str) -> Result<Option<Self>, Fault> {
        let Some(raw) = reader.optional(url, 4096)? else {
            return Ok(None);
        };
        let held: Self = Wire::parse(&raw)?;
        Wire::require(
            held.encode(product)? == raw,
            "family selector is not canonical",
        )?;
        Ok(Some(held))
    }
}
