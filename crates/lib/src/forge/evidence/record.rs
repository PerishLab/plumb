use super::super::Digest;
use super::Selection;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Record(Parts);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    schema: u32,
    selection: Selection,
}

impl Record {
    pub fn new(selection: Selection) -> Self {
        Self(Parts {
            schema: 1,
            selection,
        })
    }

    pub fn selection(&self) -> &Selection {
        &self.0.selection
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|error| error.to_string())
    }

    pub fn digest(&self) -> Result<Digest, String> {
        Digest::read(&crate::depot::sha(&self.encode()?))
    }
}

impl<'de> Deserialize<'de> for Record {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        let parts = Parts::deserialize(reader)?;
        if parts.schema != 1 {
            return Err(serde::de::Error::custom("unsupported lane evidence schema"));
        }
        Ok(Self(parts))
    }
}
