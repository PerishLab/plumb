use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Atom(String);

impl Atom {
    pub fn read(value: &str) -> Result<Self, String> {
        super::name::atom(value)?;
        Ok(Self(value.into()))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Atom {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        let value = String::deserialize(reader)?;
        Self::read(&value).map_err(serde::de::Error::custom)
    }
}
