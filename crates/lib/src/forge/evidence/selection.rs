use super::super::{Address, Artifact};
use super::Context;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Policy {
    Exact,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Reason {
    Missing,
    Unsupported,
    Denied,
    Conflict,
    Unavailable,
    Unverified,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "lowercase", deny_unknown_fields)]
pub enum Observation {
    Matched {
        actual: Address,
        artifact: Box<Artifact>,
    },
    Unmet {
        reason: Reason,
    },
    Unknown {
        reason: Reason,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Selection(Parts);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    policy: Policy,
    context: Context,
    observation: Observation,
}

impl Selection {
    pub fn new(context: Context, observation: Observation) -> Result<Self, String> {
        checked(Parts {
            policy: Policy::Exact,
            context,
            observation,
        })
    }

    pub fn policy(&self) -> Policy {
        self.0.policy
    }

    pub fn context(&self) -> &Context {
        &self.0.context
    }

    pub fn observation(&self) -> &Observation {
        &self.0.observation
    }
}

impl<'de> Deserialize<'de> for Selection {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        checked(Parts::deserialize(reader)?).map_err(serde::de::Error::custom)
    }
}

fn checked(parts: Parts) -> Result<Selection, String> {
    match &parts.observation {
        Observation::Matched { actual, .. } if actual != parts.context.requested() => {
            return Err("lane selection permits only the exact requested target".into());
        }
        Observation::Unmet { reason } if uncertain(*reason) => {
            return Err("uncertain lane evidence cannot prove an unmet selection".into());
        }
        Observation::Unknown { reason } if !uncertain(*reason) => {
            return Err("unknown lane evidence needs an uncertain reason".into());
        }
        _ => {}
    }
    Ok(Selection(parts))
}

fn uncertain(reason: Reason) -> bool {
    matches!(reason, Reason::Unavailable | Reason::Unverified)
}
