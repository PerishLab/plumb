use super::super::{Inspector, Reference, wire::Wire};
use super::History;
use crate::windtun::qualification::{Fault, Read};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Discovery {
    pub schema: String,
    pub reference: Reference,
}

impl Discovery {
    pub const SCHEMA: &str = "wharf.release.semantics.history.head/v1";

    pub fn url(product: &str) -> Result<String, Fault> {
        Wire::name(product)?;
        Ok(format!(
            "https://releases.{product}.perish.uk/v1/semantics/history/latest.json"
        ))
    }

    pub fn encode(&self, product: &str) -> Result<Vec<u8>, Fault> {
        Wire::require(self.schema == Self::SCHEMA, "unsupported history discovery")?;
        self.reference.validate(product)?;
        Wire::canonical(self)
    }

    pub(super) fn read(reader: &impl Read, product: &str) -> Result<Option<Self>, Fault> {
        let Some(raw) = reader.optional(&Self::url(product)?, 4096)? else {
            return Ok(None);
        };
        let held: Self = Wire::parse(&raw)?;
        Wire::require(
            held.encode(product)? == raw,
            "history discovery is not canonical",
        )?;
        Ok(Some(held))
    }
}

impl<R: Read> Inspector<R> {
    pub fn discover(&self, product: &str) -> Result<History, Fault> {
        let authority = self.current()?;
        let head = Discovery::read(&self.0, product)?
            .ok_or_else(|| Fault::unknown("current retained history is undiscovered"))?;
        let mut history = self.history(&head.reference, product)?;
        Wire::require(
            Discovery::read(&self.0, product)? == Some(head) && self.current()? == authority,
            "history discovery or authority moved during observation",
        )?;
        history.discovery = Some(Discovery::url(product)?);
        Ok(history)
    }
}
