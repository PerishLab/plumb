use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

const SCHEMA: &str = "plumb.workflow-inventory/v1";

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Keys {
    pub workload: String,
    pub proof: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publication: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    #[serde(rename = "type")]
    pub kind: String,
    pub source: String,
}

pub struct Context<'a> {
    world: &'a BTreeMap<String, String>,
    identity: &'a BTreeMap<String, String>,
}

pub struct Verdict {
    pub decision: &'static str,
    pub reason: &'static str,
    pub source: Source,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    #[serde(default)]
    schema: String,
    #[serde(default)]
    records: Vec<Record>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    action: String,
    workload: String,
    #[serde(default)]
    proof: Option<String>,
    #[serde(default)]
    publication: Option<String>,
    source: Source,
}

#[derive(PartialEq)]
struct Match<'a> {
    action: &'a str,
    workload: &'a str,
    publication: Option<&'a str>,
    kind: &'a str,
}

impl<'a> Context<'a> {
    pub fn new(
        world: &'a BTreeMap<String, String>,
        identity: &'a BTreeMap<String, String>,
    ) -> Self {
        Self { world, identity }
    }

    pub fn keys(&self, action: &str, workload: &str) -> Keys {
        let proof = digest(action, workload, self.world);
        let publication =
            (!self.identity.is_empty()).then(|| digest(action, &proof, self.identity));
        Keys {
            workload: workload.to_string(),
            proof,
            publication,
        }
    }
}

impl Inventory {
    pub fn read(path: Option<&Path>) -> Result<Self, String> {
        let Some(path) = path else {
            return Ok(Self::default());
        };
        let text = std::fs::read_to_string(path).map_err(|error| {
            format!("cannot read workflow inventory {}: {error}", path.display())
        })?;
        let held: Self = serde_json::from_str(&text).map_err(|error| {
            format!(
                "cannot parse workflow inventory {}: {error}",
                path.display()
            )
        })?;
        held.validate()?;
        Ok(held)
    }

    pub fn resolve(&self, action: &str, keys: &Keys) -> Result<Verdict, String> {
        if let Some(publication) = &keys.publication
            && let Some(record) =
                self.source(Match::new(action, keys, Some(publication), "url"), None)?
        {
            return Ok(Verdict {
                decision: "skip",
                reason: "publication-held",
                source: record.source.clone(),
            });
        }
        if keys.publication.is_none()
            && let Some(record) =
                self.source(Match::new(action, keys, None, "url"), Some(&keys.proof))?
        {
            return Ok(Verdict {
                decision: "reuse",
                reason: "proof-held",
                source: record.source.clone(),
            });
        }
        if let Some(record) = self.source(
            Match::new(action, keys, None, "workload"),
            Some(&keys.proof),
        )? {
            return Ok(Verdict {
                decision: if keys.publication.is_some() {
                    "run"
                } else {
                    "reuse"
                },
                reason: if keys.publication.is_some() {
                    "publication-moved"
                } else {
                    "proof-held"
                },
                source: record.source.clone(),
            });
        }
        if let Some(record) = self.source(Match::new(action, keys, None, "workload"), None)? {
            return Ok(Verdict {
                decision: "run",
                reason: "proof-moved",
                source: record.source.clone(),
            });
        }
        Ok(Verdict {
            decision: "run",
            reason: "record-absent",
            source: Source::none(),
        })
    }

    fn source<'a>(
        &'a self,
        key: Match<'_>,
        proof: Option<&str>,
    ) -> Result<Option<&'a Record>, String> {
        let mut found = self
            .records
            .iter()
            .filter(|record| record.key() == key)
            .filter(|record| proof.is_none() || record.proof.as_deref() == proof);
        let first = found.next();
        if let Some(first) = first
            && found.any(|record| record.source != first.source)
        {
            return Err(format!(
                "workflow inventory ambiguously records {} {}",
                key.action, key.kind
            ));
        }
        Ok(first)
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA {
            return Err(format!("workflow inventory schema must be {SCHEMA}"));
        }
        for record in &self.records {
            if record.action.trim().is_empty() {
                return Err("workflow inventory record names no action".to_string());
            }
            hash(&record.workload)?;
            if let Some(proof) = &record.proof {
                hash(proof)?;
            }
            if let Some(publication) = &record.publication {
                hash(publication)?;
            }
            if !matches!(record.source.kind.as_str(), "url" | "workload")
                || record.source.source.trim().is_empty()
            {
                return Err(format!(
                    "workflow inventory record {} has an invalid source",
                    record.action
                ));
            }
        }
        Ok(())
    }
}

impl Record {
    fn key(&self) -> Match<'_> {
        Match {
            action: &self.action,
            workload: &self.workload,
            publication: self.publication.as_deref(),
            kind: &self.source.kind,
        }
    }
}

impl<'a> Match<'a> {
    fn new(action: &'a str, keys: &'a Keys, publication: Option<&'a str>, kind: &'a str) -> Self {
        Self {
            action,
            workload: &keys.workload,
            publication,
            kind,
        }
    }
}

impl Source {
    pub fn none() -> Self {
        Self {
            kind: "none".to_string(),
            source: String::new(),
        }
    }
}

fn digest(action: &str, input: &str, fields: &BTreeMap<String, String>) -> String {
    let mut sponge = Sha256::new();
    sponge.update(action.as_bytes());
    sponge.update([0]);
    sponge.update(input.as_bytes());
    sponge.update([0]);
    for (name, value) in fields {
        sponge.update(name.as_bytes());
        sponge.update([0]);
        sponge.update(value.as_bytes());
        sponge.update([0]);
    }
    format!("{:x}", sponge.finalize())
}

fn hash(value: &str) -> Result<(), String> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!(
            "workflow inventory carries an invalid hash {value:?}"
        ))
    }
}
