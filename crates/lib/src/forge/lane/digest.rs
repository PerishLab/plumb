use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Digest(String);

impl Digest {
    pub fn read(value: &str) -> Result<Self, String> {
        hex(value, 64)?;
        Ok(Self(value.into()))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        let value = String::deserialize(reader)?;
        Self::read(&value).map_err(serde::de::Error::custom)
    }
}

pub(super) fn hex(value: &str, size: usize) -> Result<(), String> {
    if value.len() != size
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(format!("lane identity needs {size} lowercase hex digits"));
    }
    Ok(())
}
