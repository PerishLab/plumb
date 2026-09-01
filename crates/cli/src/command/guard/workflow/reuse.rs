use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

const SCHEMA: &str = "plumb.workflow-inventory/v1";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
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
    pub depot: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    #[serde(default)]
    pub(super) schema: String,
    #[serde(default)]
    pub(super) records: Vec<Record>,
    #[serde(skip)]
    pub(super) base: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub(super) action: String,
    pub(super) workload: String,
    #[serde(default)]
    pub(super) proof: Option<String>,
    #[serde(default)]
    pub(super) publication: Option<String>,
    pub(super) source: Source,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) depot: Option<serde_json::Value>,
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
    pub fn empty() -> Self {
        Self {
            schema: SCHEMA.to_string(),
            records: Vec::new(),
            base: None,
        }
    }

    pub fn read(path: Option<&Path>) -> Result<Self, String> {
        let Some(path) = path else {
            return Ok(Self::empty());
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
        let held = self.enrich(action, keys)?;
        held.local(action, keys)
    }

    fn local(&self, action: &str, keys: &Keys) -> Result<Verdict, String> {
        if let Some(publication) = &keys.publication
            && let Some(record) =
                self.source(Match::new(action, keys, Some(publication), "url"), None)?
        {
            return Ok(Verdict {
                decision: "skip",
                reason: "publication-held",
                source: record.source.clone(),
                depot: record.depot.clone(),
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
                depot: None,
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
                depot: None,
            });
        }
        if let Some(record) = self.source(Match::new(action, keys, None, "workload"), None)? {
            return Ok(Verdict {
                decision: "run",
                reason: "proof-moved",
                source: record.source.clone(),
                depot: None,
            });
        }
        Ok(Verdict {
            decision: "run",
            reason: "record-absent",
            source: Source::none(),
            depot: None,
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
            && found.any(|record| record.source != first.source || record.depot != first.depot)
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
            record.valid()?;
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

pub(super) fn hash(value: &str) -> Result<(), String> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!(
            "workflow inventory carries an invalid hash {value:?}"
        ))
    }
}
