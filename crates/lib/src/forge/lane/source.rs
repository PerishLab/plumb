use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Source(Parts);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    repository: String,
    commit: String,
    tree: String,
}

impl Source {
    pub fn new(repository: String, commit: String, tree: String) -> Result<Self, String> {
        checked(Parts {
            repository,
            commit,
            tree,
        })
    }

    pub fn repository(&self) -> &str {
        &self.0.repository
    }

    pub fn commit(&self) -> &str {
        &self.0.commit
    }

    pub fn tree(&self) -> &str {
        &self.0.tree
    }
}

impl<'de> Deserialize<'de> for Source {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        checked(Parts::deserialize(reader)?).map_err(serde::de::Error::custom)
    }
}

fn checked(value: Parts) -> Result<Source, String> {
    super::address::repository(&value.repository)?;
    super::digest::hex(&value.commit, 40)?;
    super::digest::hex(&value.tree, 40)?;
    Ok(Source(value))
}
