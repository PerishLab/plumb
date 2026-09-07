use super::reuse::{Inventory, Keys, Record, Source, Verdict, hash};
use plumb::rule::{Production, Receipt};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read as _;
use std::path::Path;

impl Inventory {
    pub(super) fn verified(
        &self,
        action: &str,
        keys: &Keys,
        contract: Option<&Production>,
    ) -> Result<(Verdict, Option<Receipt>), String> {
        let Some(contract) = contract else {
            return self.resolve(action, keys).map(|verdict| (verdict, None));
        };
        contract.digest()?;
        let mut held = self.enrich(action, keys)?;
        let valid = |record: &Record| {
            record.receipt.as_ref().is_some_and(|receipt| {
                contract.verify(receipt).is_ok()
                    && record.source.kind == "workload"
                    && record
                        .source
                        .source
                        .ends_with(&format!("/workloads/{}.tgz", receipt.artifact))
            })
        };
        held.records.retain(valid);
        held.direct.retain(valid);
        let verdict = held.local(action, keys)?;
        let receipt = held
            .direct
            .iter()
            .chain(&held.records)
            .filter(|record| record.action == action && record.workload == keys.workload)
            .find(|record| {
                record.proof.as_deref() == Some(&keys.proof) && record.source == verdict.source
            })
            .and_then(|record| record.receipt.clone());
        Ok((verdict, receipt))
    }
}

impl Record {
    pub(super) fn equivalent(&self, wanted: &Self, contract: Option<&Production>) -> bool {
        let Some(contract) = contract else {
            return self == wanted;
        };
        let (Some(held), Some(produced)) = (&self.receipt, &wanted.receipt) else {
            return false;
        };
        if contract.verify(held).is_err()
            || contract.verify(produced).is_err()
            || held.artifact != produced.artifact
        {
            return false;
        }
        let mut historical = self.clone();
        historical.receipt = wanted.receipt.clone();
        historical == *wanted
    }

    pub(super) fn encode(&self) -> Result<Vec<u8>, String> {
        let mut body = serde_json::to_vec_pretty(self)
            .map_err(|error| format!("cannot encode workflow record: {error}"))?;
        body.push(b'\n');
        Ok(body)
    }

    pub(super) fn decode(body: &[u8]) -> Result<Self, String> {
        let held: Self = serde_json::from_slice(body)
            .map_err(|error| format!("cannot parse workflow record: {error}"))?;
        held.valid()?;
        Ok(held)
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
            receipt: None,
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
            receipt: None,
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
