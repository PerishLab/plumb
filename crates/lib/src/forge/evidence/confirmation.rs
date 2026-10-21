use super::super::operation::{Authorization, Operation, Outcome, Request};
use super::{Reference, Verified};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Confirmation(Parts);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    original: Reference,
    authorization: Reference,
    operation: Operation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Confirmed {
    reference: Reference,
    confirmation: Confirmation,
}

impl Confirmation {
    pub fn new(
        original: Reference,
        authorization: Reference,
        operation: Operation,
    ) -> Result<Self, String> {
        checked(Parts {
            original,
            authorization,
            operation,
        })
    }

    pub fn original(&self) -> &Reference {
        &self.0.original
    }

    pub fn authorization(&self) -> &Reference {
        &self.0.authorization
    }

    pub fn operation(&self) -> &Operation {
        &self.0.operation
    }
}

impl<'de> Deserialize<'de> for Confirmation {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        checked(Parts::deserialize(reader)?).map_err(serde::de::Error::custom)
    }
}

impl Verified {
    pub fn confirm(&self, request: &Request, original: &Verified) -> Result<Confirmed, String> {
        let confirmation = self
            .record()
            .confirmation()
            .ok_or("lane confirmation needs explicit confirmation evidence")?;
        if confirmation.original() != original.reference()
            || self.reference().authority() != original.reference().authority()
        {
            return Err("lane confirmation differs from the original publication reference".into());
        }
        let prior = original
            .record()
            .operation()
            .ok_or("lane confirmation needs an original unknown operation")?;
        let Outcome::Unknown { .. } = prior.outcome() else {
            return Err("lane confirmation needs an original unknown operation".into());
        };
        let operation = confirmation.operation();
        if operation.request() != request
            || prior.request() != request
            || prior.assessment() != operation.assessment()
        {
            return Err(
                "lane confirmation differs from the exact original unknown operation".into(),
            );
        }
        Ok(Confirmed {
            reference: self.reference().clone(),
            confirmation: confirmation.clone(),
        })
    }
}

impl Confirmed {
    pub fn reference(&self) -> &Reference {
        &self.reference
    }

    pub fn confirmation(&self) -> &Confirmation {
        &self.confirmation
    }
}

fn checked(parts: Parts) -> Result<Confirmation, String> {
    if parts.original == parts.authorization
        || parts.original.authority() != parts.authorization.authority()
    {
        return Err("lane confirmation needs distinct original and new public authorization references under one publication authority".into());
    }
    if matches!(parts.operation.assessment().authorization(), Authorization::Granted { evidence } if evidence == &parts.authorization)
    {
        return Err(
            "lane confirmation cannot reuse its original admission reference as new authorization"
                .into(),
        );
    }
    if !parts.operation.request().intent().mutates()
        || !matches!(parts.operation.outcome(), Outcome::Applied { .. })
    {
        return Err("lane confirmation needs an exact applied mutation observation".into());
    }
    Ok(Confirmation(parts))
}
