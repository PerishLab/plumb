use super::identity::Identity;
use super::{Content, Material, Operation, Provider, Request, State};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Verified,
    Degraded,
    Failed,
    Discarded,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub schema: String,
    pub request: String,
    pub intent: String,
    pub outcome: Outcome,
    pub provider: Provider,
    pub content: Content,
    #[serde(deserialize_with = "super::nullable")]
    pub failure: Option<String>,
}

impl Observation {
    pub fn read(bytes: &[u8], request: &Request) -> Result<Self, String> {
        let held: Self = super::read(bytes)?;
        held.validate(request)?;
        Ok(held)
    }

    pub fn validate(&self, request: &Request) -> Result<(), String> {
        if self.schema != "wharf.preview.result/v1" {
            return Err("unsupported Preview result schema".into());
        }
        if self.request != request.request || self.intent != request.digest()? {
            return Err("Preview result does not bind this exact request".into());
        }
        self.provider.validate()?;
        self.content.validate()?;
        if let Some(failure) = &self.failure {
            Identity(failure).text(2048)?;
        }
        if self.outcome != Outcome::Unknown
            && (!self.provider.quiescent || self.provider.state == State::Unknown)
        {
            return Err("Preview uncertain writer/provider cannot settle this request".into());
        }
        match self.outcome {
            Outcome::Verified => self.verified(request),
            Outcome::Discarded => self.discarded(request),
            Outcome::Degraded => self.degraded(request),
            Outcome::Failed | Outcome::Unknown => self.failure(),
        }
    }

    pub fn terminal(&self, request: &Request) -> Result<bool, String> {
        self.validate(request)?;
        Ok(self.outcome != Outcome::Unknown)
    }

    fn failure(&self) -> Result<(), String> {
        if self.failure.is_none() {
            return Err("Preview incomplete result must explain its failure".into());
        }
        Ok(())
    }

    fn verified(&self, request: &Request) -> Result<(), String> {
        let held = self
            .provider
            .deployment
            .as_ref()
            .ok_or("Preview verification needs a deployment")?;
        if request.operation != Operation::Apply
            || self.content.state != Material::Verified
            || self.failure.is_some()
        {
            return Err(
                "Preview verified result needs apply and verified bytes without failure".into(),
            );
        }
        if self.content.digest.as_ref() != Some(&held.digest) || held.request != request.request {
            return Err(
                "Preview verification needs this deployment and matching byte digest".into(),
            );
        }
        Ok(())
    }

    fn discarded(&self, request: &Request) -> Result<(), String> {
        if request.operation != Operation::Discard
            || self.provider.state != State::Absent
            || self.failure.is_some()
        {
            return Err(
                "Preview discard needs explicit discard and provider absence without failure"
                    .into(),
            );
        }
        if self.content.state != Material::Gone || self.content.digest.is_some() {
            return Err("Preview discard needs confirmed content disappearance".into());
        }
        Ok(())
    }

    fn degraded(&self, request: &Request) -> Result<(), String> {
        self.failure()?;
        let held = self
            .provider
            .deployment
            .as_ref()
            .ok_or("Preview degraded result needs a deployment")?;
        if request.operation != Operation::Apply
            || self.content.state != Material::Unverified
            || held.request != request.request
        {
            return Err("Preview degraded result needs this unverified apply deployment".into());
        }
        Ok(())
    }
}
