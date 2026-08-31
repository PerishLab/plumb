use super::reuse::{Inventory, Keys, Record, Source, hash};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read as _;
use std::path::Path;
use std::process::Output;

#[derive(PartialEq)]
struct Key<'a> {
    action: &'a str,
    workload: &'a str,
    publication: Option<&'a str>,
    kind: &'a str,
}

impl Inventory {
    pub(super) fn record(&mut self, record: Record) -> Result<(), String> {
        record.valid()?;
        self.records
            .retain(|held| held.identity() != record.identity());
        self.records.push(record);
        self.records.sort_by(|left, right| {
            (
                left.action.as_str(),
                left.workload.as_str(),
                left.publication.as_deref(),
                left.source.kind.as_str(),
            )
                .cmp(&(
                    right.action.as_str(),
                    right.workload.as_str(),
                    right.publication.as_deref(),
                    right.source.kind.as_str(),
                ))
        });
        Ok(())
    }

    pub(super) fn encode(&self) -> Result<Vec<u8>, String> {
        let mut body = serde_json::to_vec_pretty(self)
            .map_err(|error| format!("cannot encode workflow inventory: {error}"))?;
        body.push(b'\n');
        Ok(body)
    }
}

impl Record {
    fn identity(&self) -> Key<'_> {
        Key {
            action: &self.action,
            workload: &self.workload,
            publication: self.publication.as_deref(),
            kind: &self.source.kind,
        }
    }

    pub(super) fn workload(action: String, keys: &Keys, source: String) -> Self {
        Self {
            action,
            workload: keys.workload.clone(),
            proof: Some(keys.proof.clone()),
            publication: None,
            source: Source {
                kind: "workload".to_string(),
                source,
            },
            depot: None,
        }
    }

    pub(super) fn publication(
        action: String,
        keys: &Keys,
        source: String,
        depot: Option<serde_json::Value>,
    ) -> Option<Self> {
        keys.publication.as_ref().map(|publication| Self {
            action,
            workload: keys.workload.clone(),
            proof: Some(keys.proof.clone()),
            publication: Some(publication.clone()),
            source: Source {
                kind: "url".to_string(),
                source,
            },
            depot,
        })
    }

    pub(super) fn valid(&self) -> Result<(), String> {
        if self.action.trim().is_empty() {
            return Err("workflow inventory record names no action".to_string());
        }
        hash(&self.workload)?;
        if let Some(proof) = &self.proof {
            hash(proof)?;
        }
        if let Some(publication) = &self.publication {
            hash(publication)?;
        }
        if !matches!(self.source.kind.as_str(), "url" | "workload")
            || self.source.source.trim().is_empty()
        {
            return Err(format!(
                "workflow inventory record {} has an invalid source",
                self.action
            ));
        }
        if self.depot.is_some() && self.source.kind != "url" {
            return Err(format!(
                "workflow inventory record {} binds depot state to a non-publication source",
                self.action
            ));
        }
        if self.depot.as_ref().is_some_and(|depot| !depot.is_object()) {
            return Err(format!(
                "workflow inventory record {} carries a non-object depot binding",
                self.action
            ));
        }
        Ok(())
    }
}

pub(super) fn field(name: &str, value: String) -> Result<String, String> {
    if value.trim().is_empty() || value.contains(['\r', '\n']) {
        Err(format!("missing or invalid {name}"))
    } else {
        Ok(value)
    }
}

pub(super) fn digest(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("cannot read workload {}: {error}", path.display()))?;
    let mut sponge = Sha256::new();
    let mut block = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut block)
            .map_err(|error| format!("cannot read workload {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        sponge.update(&block[..read]);
    }
    Ok(format!("{:x}", sponge.finalize()))
}

pub(super) fn absent(output: &Output) -> bool {
    let text = String::from_utf8_lossy(&output.stderr);
    text.contains("NoSuchKey") || text.contains("Not Found") || text.contains("404")
}

pub(super) fn stale(output: &Output) -> bool {
    let text = String::from_utf8_lossy(&output.stderr);
    text.contains("PreconditionFailed") || text.contains("412")
}

pub(super) fn failure(action: &str, output: &Output) -> String {
    format!(
        "{action}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
}
