use super::super::{Digest, Name};
use super::Record;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Reference(Parts);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    authority: Name,
    digest: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Verified {
    reference: Reference,
    record: Record,
}

impl Reference {
    pub fn new(authority: Name, digest: Digest) -> Result<Self, String> {
        checked(Parts { authority, digest })
    }

    pub fn authority(&self) -> &Name {
        &self.0.authority
    }

    pub fn digest(&self) -> &Digest {
        &self.0.digest
    }

    pub fn verify(&self, observed: &Reference, bytes: &[u8]) -> Result<Verified, String> {
        if self != observed {
            return Err(
                "lane evidence retrieval reference differs from requested reference".into(),
            );
        }
        if crate::depot::sha(bytes) != self.digest().value() {
            return Err("lane evidence retrieved content digest differs".into());
        }
        let record: Record = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if record.encode()? != bytes {
            return Err("lane evidence retrieved bytes are not canonical".into());
        }
        Ok(Verified {
            reference: self.clone(),
            record,
        })
    }
}

impl<'de> Deserialize<'de> for Reference {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        checked(Parts::deserialize(reader)?).map_err(serde::de::Error::custom)
    }
}

impl Verified {
    pub fn reference(&self) -> &Reference {
        &self.reference
    }

    pub fn record(&self) -> &Record {
        &self.record
    }
}

fn checked(parts: Parts) -> Result<Reference, String> {
    if parts.authority.key().is_some() {
        return Err("lane evidence authority needs one identity atom".into());
    }
    Ok(Reference(parts))
}
