use super::identity::Identity;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Present,
    Absent,
    Unchanged,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Material {
    Verified,
    Unverified,
    Gone,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Deployment {
    pub id: String,
    pub request: String,
    pub digest: String,
    #[serde(rename = "latest_url")]
    pub latest: String,
    #[serde(rename = "exact_url")]
    pub exact: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Provider {
    pub state: State,
    #[serde(deserialize_with = "super::nullable")]
    pub deployment: Option<Deployment>,
    pub quiescent: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Content {
    pub state: Material,
    #[serde(deserialize_with = "super::nullable")]
    pub digest: Option<String>,
}

impl Deployment {
    pub fn validate(&self) -> Result<(), String> {
        Identity(&self.id).text(256)?;
        Identity(&self.request).hex(32)?;
        Identity(&self.digest).hex(64)?;
        Identity(&self.latest).origin()?;
        Identity(&self.exact).origin()?;
        if self.latest == self.exact {
            return Err("Preview latest and exact URLs must differ".into());
        }
        Ok(())
    }
}

impl Provider {
    pub fn validate(&self) -> Result<(), String> {
        if (self.state == State::Present) != self.deployment.is_some() {
            return Err("Preview present observation requires an exact deployment".into());
        }
        if let Some(deployment) = &self.deployment {
            deployment.validate()?;
        }
        Ok(())
    }
}

impl Content {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(digest) = &self.digest {
            Identity(digest).hex(64)?;
        }
        match (&self.state, &self.digest) {
            (Material::Verified, None) => Err("Preview verified bytes need a digest".into()),
            (Material::Gone, Some(_)) => Err("Preview gone bytes cannot carry a digest".into()),
            _ => Ok(()),
        }
    }
}
