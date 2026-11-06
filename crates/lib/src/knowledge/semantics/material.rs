use serde::{Deserialize, Serialize};

use super::wire::Wire;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Schema,
    Case,
    Setup,
    Helper,
    Runtime,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Material {
    pub kind: Kind,
    pub name: String,
    pub sha256: String,
    pub size: u64,
    pub executable: bool,
}

impl Material {
    pub fn new(kind: Kind, name: String, bytes: &[u8], executable: bool) -> Result<Self, String> {
        let held = Self {
            kind,
            name,
            sha256: Wire::digest(bytes),
            size: bytes.len() as u64,
            executable,
        };
        held.validate()?;
        Ok(held)
    }

    pub fn verify(&self, bytes: &[u8], executable: bool) -> Result<(), String> {
        self.validate()?;
        if self.size != bytes.len() as u64
            || self.sha256 != Wire::digest(bytes)
            || self.executable != executable
        {
            return Err(format!("material bytes or mode differ for {}", self.name));
        }
        Ok(())
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        Wire::path(&self.name)?;
        if self.kind == Kind::Case {
            Wire::name(&self.name)?;
        }
        Wire::hex(&self.sha256, 64)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub case: String,
    pub platform: String,
    pub backend: String,
}

impl Coverage {
    pub(super) fn validate(&self) -> Result<(), String> {
        Wire::name(&self.case)?;
        Wire::name(&self.platform)?;
        Wire::name(&self.backend)
    }
}
