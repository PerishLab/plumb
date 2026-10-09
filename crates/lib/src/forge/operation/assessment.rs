use super::super::evidence::Reference;
use super::super::{Capabilities, Digest};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase", deny_unknown_fields)]
pub enum Authorization {
    Granted { evidence: Reference },
    Denied { evidence: Reference },
    Unknown {},
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Writer {
    Idle,
    Active,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase", deny_unknown_fields)]
pub enum State {
    Absent {},
    Present { revision: u64, digest: Digest },
    Unknown {},
}

impl State {
    pub(super) fn validate(&self) -> Result<(), String> {
        if matches!(self, Self::Present { revision: 0, .. }) {
            return Err("lane observed revision must be positive".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Assessment(Parts);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    request: Digest,
    capabilities: Capabilities,
    conditions: Conditions,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Conditions(Observed);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Observed {
    authorization: Authorization,
    writer: Writer,
    observed: State,
}

impl Assessment {
    pub fn new(request: Digest, capabilities: Capabilities, conditions: Conditions) -> Self {
        Self(Parts {
            request,
            capabilities,
            conditions,
        })
    }

    pub fn request(&self) -> &Digest {
        &self.0.request
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.0.capabilities
    }

    pub fn authorization(&self) -> &Authorization {
        &self.0.conditions.0.authorization
    }

    pub fn writer(&self) -> Writer {
        self.0.conditions.0.writer
    }

    pub fn observed(&self) -> &State {
        &self.0.conditions.0.observed
    }
}

impl Conditions {
    pub fn new(
        authorization: Authorization,
        writer: Writer,
        observed: State,
    ) -> Result<Self, String> {
        observed.validate()?;
        Ok(Self(Observed {
            authorization,
            writer,
            observed,
        }))
    }
}

impl<'de> Deserialize<'de> for Conditions {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        let held = Observed::deserialize(reader)?;
        Self::new(held.authorization, held.writer, held.observed).map_err(serde::de::Error::custom)
    }
}
