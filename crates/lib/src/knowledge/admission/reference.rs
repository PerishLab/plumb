use crate::windtun::qualification::{Fault, Read};
use serde::{Deserialize, Serialize};

use super::wire::Wire;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

impl Reference {
    pub fn validate(&self, product: &str) -> Result<(), Fault> {
        Wire::name(product)?;
        Wire::hex(&self.sha256, 64)?;
        Wire::require(
            self.url
                == format!(
                    "https://releases.{product}.perish.uk/v1/semantics/objects/{}.json",
                    self.sha256
                ),
            "semantic reference has a foreign authority or address",
        )?;
        Wire::require(
            self.size > 0 && self.size <= Wire::LIMIT as u64,
            "semantic reference exceeds its bounds",
        )
    }

    pub fn bytes(&self, reader: &impl Read, product: &str) -> Result<Vec<u8>, Fault> {
        self.validate(product)?;
        let raw = reader.bytes(&self.url, self.size as usize)?;
        Wire::require(
            raw.len() as u64 == self.size && Wire::sha(&raw) == self.sha256,
            "semantic reference bytes differ",
        )?;
        Ok(raw)
    }
}
