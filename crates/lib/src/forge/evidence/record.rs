use super::super::Digest;
use super::super::operation::Operation;
use super::Selection;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Record(Parts);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    schema: u32,
    entry: Entry,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "lowercase",
    deny_unknown_fields
)]
pub enum Entry {
    Selection(Box<Selection>),
    Operation(Box<Operation>),
}

impl Record {
    pub fn new(selection: Selection) -> Self {
        Self(Parts {
            schema: 2,
            entry: Entry::Selection(Box::new(selection)),
        })
    }

    pub fn execution(operation: Operation) -> Self {
        Self(Parts {
            schema: 2,
            entry: Entry::Operation(Box::new(operation)),
        })
    }

    pub fn entry(&self) -> &Entry {
        &self.0.entry
    }

    pub fn selection(&self) -> Option<&Selection> {
        match self.entry() {
            Entry::Selection(selection) => Some(selection),
            _ => None,
        }
    }

    pub fn operation(&self) -> Option<&Operation> {
        match self.entry() {
            Entry::Operation(operation) => Some(operation),
            _ => None,
        }
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
        if parts.schema != 2 {
            return Err(serde::de::Error::custom("unsupported lane evidence schema"));
        }
        Ok(Self(parts))
    }
}
