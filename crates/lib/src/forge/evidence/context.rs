use super::super::{Address, Digest, Name};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Context(Parts);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    requested: Address,
    request: Digest,
    revision: u64,
    adaptor: Name,
}

impl Context {
    pub fn new(
        requested: Address,
        request: Digest,
        revision: u64,
        adaptor: Name,
    ) -> Result<Self, String> {
        checked(Parts {
            requested,
            request,
            revision,
            adaptor,
        })
    }

    pub fn requested(&self) -> &Address {
        &self.0.requested
    }

    pub fn request(&self) -> &Digest {
        &self.0.request
    }

    pub fn revision(&self) -> u64 {
        self.0.revision
    }

    pub fn adaptor(&self) -> &Name {
        &self.0.adaptor
    }
}

impl<'de> Deserialize<'de> for Context {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        checked(Parts::deserialize(reader)?).map_err(serde::de::Error::custom)
    }
}

fn checked(parts: Parts) -> Result<Context, String> {
    if parts.revision == 0 {
        return Err("lane evidence needs a positive revision".into());
    }
    if parts.adaptor.key().is_some() {
        return Err("lane evidence adaptor needs one identity atom".into());
    }
    Ok(Context(parts))
}
