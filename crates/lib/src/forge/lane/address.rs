use super::Name;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Address(Parts);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    repository: String,
    app: String,
    lane: Name,
}

impl Address {
    pub fn new(repository: String, app: String, lane: Name) -> Result<Self, String> {
        checked(Parts {
            repository,
            app,
            lane,
        })
    }

    pub fn repository(&self) -> &str {
        &self.0.repository
    }

    pub fn app(&self) -> &str {
        &self.0.app
    }

    pub fn lane(&self) -> &Name {
        &self.0.lane
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        checked(Parts::deserialize(reader)?).map_err(serde::de::Error::custom)
    }
}

fn checked(value: Parts) -> Result<Address, String> {
    repository(&value.repository)?;
    super::name::atom(&value.app)?;
    Ok(Address(value))
}

fn repository(value: &str) -> Result<(), String> {
    let parts: Vec<_> = value.split('/').collect();
    if parts.len() != 2 || parts.iter().any(|part| part.is_empty() || part.len() > 100) {
        return Err("lane repository needs a bounded owner/name coordinate".into());
    }
    if parts.iter().any(|part| {
        !part
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    }) {
        return Err("lane repository has malformed coordinate segments".into());
    }
    Ok(())
}
