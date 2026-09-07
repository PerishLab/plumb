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
    pub(super) fn bound(&self, wanted: &Self) -> bool {
        if self.binding.is_none() {
            return false;
        }
        self.binding == wanted.binding && self.source == wanted.source && self.depot == wanted.depot
    }

    pub(super) fn binding(&self, wanted: &Self) -> Result<(), String> {
        if self.bound(wanted) {
            Ok(())
        } else {
            Err("permanent publication binding drifted".into())
        }
    }

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
            binding: None,
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
            binding: None,
        })
    }

    pub(super) fn valid(&self) -> Result<(), String> {
        if let Some(binding) = &self.binding {
            hash(binding)?;
            if self.source.kind != "url" || !self.source.source.starts_with("https://") {
                return Err("permanent publication binding requires an HTTPS publication".into());
            }
        }
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

pub struct Binding {
    pub key: String,
}

impl Binding {
    pub fn new(marker: &str, resource: &str) -> Result<Self, String> {
        hash(marker)?;
        if resource.trim().is_empty() {
            return Err("publication binding requires a resource".into());
        }
        let bytes = serde_json::to_vec(&(marker, resource)).map_err(|error| error.to_string())?;
        Ok(Self {
            key: format!("{:x}", Sha256::digest(bytes)),
        })
    }

    pub fn resolve(
        &self,
        path: Option<&Path>,
        source: Option<&str>,
    ) -> Result<Option<String>, String> {
        let inventory = Inventory::read(path)?.at(source)?;
        let mut records = inventory.records;
        if let Some(base) = &inventory.base {
            let route = format!("{base}/records/binding/{}.json", self.key);
            if let Some(body) = plumb::bucket::fetch(&route)? {
                let record = Record::decode(&body)?;
                if record.binding.as_ref() != Some(&self.key) {
                    return Err("publication binding returned a different identity".into());
                }
                records.push(record);
            }
        }
        let mut held: Option<&Record> = None;
        for record in records
            .iter()
            .filter(|record| record.binding.as_ref() == Some(&self.key))
        {
            if held.is_some_and(|held| !held.bound(record)) {
                return Err("permanent publication binding is ambiguous".into());
            }
            held = Some(record);
        }
        Ok(held.map(|record| record.source.source.clone()))
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
