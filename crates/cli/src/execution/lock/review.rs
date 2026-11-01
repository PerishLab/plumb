use super::reference;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

#[derive(Clone, Copy, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Mode {
    Default,
    #[serde(rename = "all-features")]
    All,
}

impl Mode {
    pub fn all(self) -> bool {
        self == Self::All
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Declaration {
    #[serde(default)]
    pub enforce: bool,
    pub target: String,
    pub features: Vec<Mode>,
    #[serde(default)]
    pub review: Vec<Review>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Review {
    pub reference: String,
    pub role: Role,
    pub protocol: String,
    pub hold: String,
    pub release: String,
    pub verification: String,
    pub evidence: BTreeMap<String, String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Role {
    Carrier,
    Interoperability,
    Compatibility,
}

impl Declaration {
    pub fn read(root: &Path) -> Result<Self, String> {
        Self::optional(root)?
            .ok_or_else(|| "lock check requires an explicit [guard.lock] declaration".into())
    }

    pub fn optional(root: &Path) -> Result<Option<Self>, String> {
        let Some(held) = Self::parse(root)? else {
            return Ok(None);
        };
        held.validate(root)?;
        Ok(Some(held))
    }

    pub fn enforced(root: &Path) -> Result<Option<Self>, String> {
        let Some(held) = Self::parse(root)? else {
            return Ok(None);
        };
        if !held.enforce {
            return Ok(None);
        }
        held.validate(root)?;
        Ok(Some(held))
    }

    fn parse(root: &Path) -> Result<Option<Self>, String> {
        let text = match std::fs::read_to_string(root.join("plumb.toml")) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        let document: toml::Table = text
            .parse()
            .map_err(|error| format!("invalid lock policy: {error}"))?;
        let Some(value) = document.get("guard").and_then(|value| value.get("lock")) else {
            return Ok(None);
        };
        let held: Self = value
            .clone()
            .try_into()
            .map_err(|error| format!("invalid guard.lock: {error}"))?;
        Ok(Some(held))
    }

    fn validate(&self, root: &Path) -> Result<(), String> {
        if self.target.trim().is_empty() || self.features.is_empty() {
            return Err("guard.lock needs a target and at least one feature selection".into());
        }
        let unique: BTreeSet<_> = self.features.iter().collect();
        if unique.len() != self.features.len() {
            return Err("guard.lock repeats a feature selection".into());
        }
        let mut references = BTreeSet::new();
        for review in &self.review {
            if !hex(&review.reference) || !references.insert(&review.reference) {
                return Err("lock reviews need unique SHA-256 reference identities".into());
            }
            review.validate(root)?;
        }
        Ok(())
    }
}

impl Review {
    fn validate(&self, root: &Path) -> Result<(), String> {
        if [
            &self.protocol,
            &self.hold,
            &self.release,
            &self.verification,
        ]
        .iter()
        .any(|value| value.trim().is_empty())
            || self.evidence.is_empty()
        {
            return Err(
                "lock review needs protocol, hold, release, verification and source evidence"
                    .into(),
            );
        }
        for (name, expected) in &self.evidence {
            if Path::new(name)
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
                || name.contains(['\\', '*', '?', '[', ']'])
                || !hex(expected)
            {
                return Err(
                    "lock review evidence needs exact relative files and SHA-256 digests".into(),
                );
            }
            let (actual, digest) = reference::file(root, name)?;
            if &actual != name || &digest != expected {
                return Err(format!(
                    "lock review evidence is stale or redirected: {name}"
                ));
            }
        }
        Ok(())
    }
}

fn hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
