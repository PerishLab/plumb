use super::evidence::{Reason, Receipt, Verified};
use super::operation::{Assessment, Authorization, Request};
use super::{Address, Capabilities, Digest, Name};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Authorities(Seats);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Seats {
    authorization: Name,
    publication: Name,
}

impl Authorities {
    pub fn new(authorization: Name, publication: Name) -> Result<Self, String> {
        for name in [&authorization, &publication] {
            if name.key().is_some() {
                return Err("lane authority needs one identity atom".into());
            }
        }
        Ok(Self(Seats {
            authorization,
            publication,
        }))
    }

    pub fn authorization(&self) -> &Name {
        &self.0.authorization
    }

    pub fn publication(&self) -> &Name {
        &self.0.publication
    }
}

impl<'de> Deserialize<'de> for Authorities {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        let seats = Seats::deserialize(reader)?;
        Self::new(seats.authorization, seats.publication).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Declaration(Parts);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    target: Address,
    adaptor: Name,
    capabilities: Capabilities,
    authorities: Authorities,
}

impl Declaration {
    pub fn new(
        target: Address,
        adaptor: Name,
        capabilities: Capabilities,
        authorities: Authorities,
    ) -> Result<Self, String> {
        if adaptor.key().is_some() {
            return Err("lane declaration adaptor needs one identity atom".into());
        }
        Ok(Self(Parts {
            target,
            adaptor,
            capabilities,
            authorities,
        }))
    }

    pub fn target(&self) -> &Address {
        &self.0.target
    }

    pub fn adaptor(&self) -> &Name {
        &self.0.adaptor
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.0.capabilities
    }

    pub fn authorities(&self) -> &Authorities {
        &self.0.authorities
    }

    pub fn digest(&self) -> Result<Digest, String> {
        let bytes = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        Digest::read(&crate::depot::sha(&bytes))
    }

    pub fn admit(&self, request: &Request, assessment: &Assessment) -> Result<(), Reason> {
        if request.context().requested() != self.target()
            || request.context().adaptor() != self.adaptor()
        {
            return Err(Reason::Conflict);
        }
        if assessment.capabilities() != self.capabilities() {
            return Err(Reason::Conflict);
        }
        match assessment.authorization() {
            Authorization::Granted { evidence } | Authorization::Denied { evidence }
                if evidence.authority() != self.authorities().authorization() =>
            {
                return Err(Reason::Unverified);
            }
            _ => {}
        }
        request.admit(assessment)
    }

    pub fn settle(&self, request: &Request, verified: &Verified) -> Result<Receipt, String> {
        if verified.reference().authority() != self.authorities().publication() {
            return Err(
                "lane completion evidence belongs to an undeclared publication authority".into(),
            );
        }
        let receipt = verified.settle(request)?;
        self.admit(request, receipt.operation().assessment())
            .map_err(|reason| format!("lane declaration refuses completion: {reason:?}"))?;
        Ok(receipt)
    }
}

impl<'de> Deserialize<'de> for Declaration {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        let parts = Parts::deserialize(reader)?;
        Self::new(
            parts.target,
            parts.adaptor,
            parts.capabilities,
            parts.authorities,
        )
        .map_err(serde::de::Error::custom)
    }
}
