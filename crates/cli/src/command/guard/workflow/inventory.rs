use super::reuse::{Inventory, Keys, Record, Source, hash};

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
        }
    }

    pub(super) fn publication(action: String, keys: &Keys, source: String) -> Option<Self> {
        keys.publication.as_ref().map(|publication| Self {
            action,
            workload: keys.workload.clone(),
            proof: Some(keys.proof.clone()),
            publication: Some(publication.clone()),
            source: Source {
                kind: "url".to_string(),
                source,
            },
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
        Ok(())
    }
}
