use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    pub schema: String,
    pub marker: String,
    pub contract: String,
    #[serde(deserialize_with = "resources")]
    pub resources: BTreeMap<String, Resource>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    pub source: String,
    pub digest: String,
}

impl Completion {
    pub fn verify(
        &self,
        marker: &str,
        contract: &str,
        resources: &BTreeSet<String>,
    ) -> Result<(), String> {
        if self.schema != "plumb.ship-completion/v1" {
            return Err("unsupported Ship completion schema".into());
        }
        if self.marker != marker || self.contract != contract {
            return Err("Ship completion differs from its marker or distribution contract".into());
        }
        hash(&self.marker)?;
        hash(&self.contract)?;
        if resources.is_empty()
            || self.resources.keys().cloned().collect::<BTreeSet<_>>() != *resources
        {
            return Err("Ship completion does not cover the exact distribution target set".into());
        }
        for resource in self.resources.values() {
            hash(&resource.digest)?;
            source(&resource.source)?;
        }
        Ok(())
    }
}

fn hash(value: &str) -> Result<(), String> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err("Ship completion carries an invalid SHA-256 digest".into())
    }
}

fn source(value: &str) -> Result<(), String> {
    let address = value
        .strip_prefix("https://")
        .ok_or("Ship completion resource requires HTTPS")?;
    let host = address.split('/').next().unwrap_or_default();
    let control = value
        .chars()
        .any(|held| held.is_whitespace() || held.is_control());
    let malformed = control || value.contains(['#', '?', '\\']);
    if host.is_empty() || host.contains('@') || malformed {
        return Err("Ship completion resource requires an uncredentialed immutable URL".into());
    }
    Ok(())
}

fn resources<'de, D>(reader: D) -> Result<BTreeMap<String, Resource>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    reader.deserialize_map(Map)
}

struct Map;
impl<'de> serde::de::Visitor<'de> for Map {
    type Value = BTreeMap<String, Resource>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("unique distribution resources")
    }

    fn visit_map<A>(self, mut reader: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut resources = BTreeMap::new();
        while let Some((key, value)) = reader.next_entry::<String, Resource>()? {
            if resources.insert(key, value).is_some() {
                return Err(serde::de::Error::custom("duplicate distribution resource"));
            }
        }
        Ok(resources)
    }
}
