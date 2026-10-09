use super::super::Artifact;
use super::super::evidence::Reason;
use super::{Assessment, Intent, Request, State, Writer};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase", deny_unknown_fields)]
pub enum Material {
    Present { artifact: Box<Artifact> },
    Absent {},
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "lowercase", deny_unknown_fields)]
pub enum Outcome {
    Applied { material: Material, writer: Writer },
    Refused { reason: Reason },
    Unknown { reason: Reason },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Operation(Parts);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    request: Request,
    assessment: Assessment,
    outcome: Outcome,
}

impl Operation {
    pub fn new(request: Request, assessment: Assessment, outcome: Outcome) -> Result<Self, String> {
        checked(Parts {
            request,
            assessment,
            outcome,
        })
    }

    pub fn request(&self) -> &Request {
        &self.0.request
    }

    pub fn assessment(&self) -> &Assessment {
        &self.0.assessment
    }

    pub fn outcome(&self) -> &Outcome {
        &self.0.outcome
    }
}

impl<'de> Deserialize<'de> for Operation {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        checked(Parts::deserialize(reader)?).map_err(serde::de::Error::custom)
    }
}

fn checked(parts: Parts) -> Result<Operation, String> {
    let admission = parts.request.admit(&parts.assessment);
    match &parts.outcome {
        Outcome::Applied { material, writer } => {
            admission.map_err(|reason| format!("lane operation was not admitted: {reason:?}"))?;
            if parts.request.intent().mutates() && *writer != Writer::Idle {
                return Err("uncertain or active writer cannot settle a lane mutation".into());
            }
            material.validate(&parts.request, &parts.assessment)?;
        }
        Outcome::Refused { reason } if uncertain(*reason) => {
            return Err("uncertain lane operation cannot prove refusal".into());
        }
        Outcome::Unknown { reason } if !uncertain(*reason) => {
            return Err("unknown lane operation needs an uncertain reason".into());
        }
        Outcome::Refused { reason } | Outcome::Unknown { reason } => {
            if admission.is_err_and(|held| held != *reason) {
                return Err("lane outcome does not preserve its admission refusal".into());
            }
        }
    }
    Ok(Operation(parts))
}

impl Material {
    fn validate(&self, request: &Request, assessment: &Assessment) -> Result<(), String> {
        let valid = match (request.intent(), self) {
            (Intent::Inspect {}, Self::Absent {}) => assessment.observed() == &State::Absent {},
            (Intent::Inspect {}, Self::Present { artifact }) => {
                matches!(assessment.observed(), State::Present { digest, .. } if digest == artifact.digest())
            }
            (Intent::Build { source, build }, Self::Present { artifact }) => {
                artifact.source() == source && artifact.build() == Some(build)
            }
            (
                Intent::Publish { artifact }
                | Intent::Install { artifact }
                | Intent::Deploy { artifact },
                Self::Present { artifact: actual },
            ) => artifact == actual,
            (Intent::Uninstall {} | Intent::Dispose {}, Self::Absent {}) => true,
            _ => false,
        };
        if !valid {
            return Err("lane operation material does not settle this exact intent".into());
        }
        Ok(())
    }
}

fn uncertain(reason: Reason) -> bool {
    matches!(reason, Reason::Unavailable | Reason::Unverified)
}
