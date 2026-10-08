use super::identity::Identity;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Apply,
    Inspect,
    Discard,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub commit: String,
    pub tree: String,
    pub declaration: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub operation: Operation,
    pub repository: String,
    pub app: String,
    pub name: String,
    pub revision: u64,
    pub request: String,
    #[serde(deserialize_with = "super::nullable")]
    pub source: Option<Source>,
    pub registration: String,
    pub caller: String,
    pub workflow: String,
}

impl Source {
    pub fn validate(&self) -> Result<(), String> {
        Identity(&self.commit).hex(40)?;
        Identity(&self.tree).hex(40)?;
        Identity(&self.declaration).hex(64)
    }
}

impl Request {
    pub fn read(bytes: &[u8]) -> Result<Self, String> {
        let held: Self = super::read(bytes)?;
        held.validate()?;
        Ok(held)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "wharf.preview.request/v1" {
            return Err("unsupported Preview request schema".into());
        }
        Identity(&self.repository).repository()?;
        Identity(&self.app).slug()?;
        Identity(&self.name).slug()?;
        Identity(&self.request).hex(32)?;
        Identity(&self.registration).hex(64)?;
        Identity(&self.caller).text(256)?;
        Identity(&self.workflow).text(256)?;
        match (&self.operation, &self.source) {
            (Operation::Apply, Some(source)) => source.validate(),
            (Operation::Inspect | Operation::Discard, None) => Ok(()),
            _ => Err("Preview apply needs source; inspect/discard must not carry source".into()),
        }
    }

    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        super::encode(self)
    }

    pub fn digest(&self) -> Result<String, String> {
        Ok(format!("{:x}", Sha256::digest(self.bytes()?)))
    }
}
