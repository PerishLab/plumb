use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Name(String);

impl Name {
    pub fn read(value: &str) -> Result<Self, String> {
        let mut parts = value.split('.');
        atom(parts.next().unwrap_or_default())?;
        if let Some(key) = parts.next() {
            atom(key)?;
        }
        if parts.next().is_some() {
            return Err("lane name admits only a family and optional key".into());
        }
        Ok(Self(value.into()))
    }

    pub fn value(&self) -> &str {
        &self.0
    }

    pub fn family(&self) -> &str {
        self.0.split_once('.').map_or(&self.0, |(family, _)| family)
    }

    pub fn key(&self) -> Option<&str> {
        self.0.split_once('.').map(|(_, key)| key)
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        let value = String::deserialize(reader)?;
        Self::read(&value).map_err(serde::de::Error::custom)
    }
}

pub(super) fn atom(value: &str) -> Result<(), String> {
    if value.len() > 48 || !value.starts_with(|char: char| char.is_ascii_lowercase()) {
        return Err("lane atom needs a lowercase slug of at most 48 bytes".into());
    }
    if value.split('-').any(|part| {
        part.is_empty()
            || !part
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    }) {
        return Err("lane atom has malformed slug segments".into());
    }
    Ok(())
}
