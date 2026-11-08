use super::super::{Inspector, Reference, policy, producer, wire::Wire};
use super::{Checkpoint, History};
use crate::windtun::qualification::{Fault, Read};
use serde_json::Value;
use std::collections::BTreeSet;

struct Held {
    reference: Reference,
    checkpoint: Checkpoint,
    raw: Vec<u8>,
    job: Value,
}

impl<R: Read> Inspector<R> {
    pub fn history(&self, reference: &Reference, product: &str) -> Result<History, Fault> {
        reference.validate(product)?;
        let authority = self.current()?;
        let repository = Wire::repository(&self.0, &format!("PerishLab/{product}"))?;
        let mut held: Vec<Held> = Vec::new();
        let mut seen = BTreeSet::new();
        let mut next = Some(reference.clone());
        let mut total = 0_u64;
        while let Some(reference) = next {
            if held.len() >= History::CHECKPOINTS
                || total.saturating_add(reference.size) > History::BYTES
            {
                return Err(Fault::unknown(
                    "retained history exceeds complete observation bounds",
                ));
            }
            Wire::require(
                seen.insert(reference.sha256.clone()),
                "retained history repeats a checkpoint",
            )?;
            let raw = reference.bytes(&self.0, product)?;
            let checkpoint = Checkpoint::parse(&raw)?;
            Wire::require(
                checkpoint.product == product && checkpoint.repository == repository,
                "retained history names another current repository authority",
            )?;
            if let Some(child) = held.last() {
                child.checkpoint.follows(&reference, &checkpoint)?;
            }
            let reader = producer::Reader(&self.0);
            reader.lineage(&checkpoint.native.commit, &authority.source.commit)?;
            reader.run(&checkpoint.native)?;
            let job = reader.role(
                &checkpoint.native,
                "[retain] semantic history",
                &format!("[retain] history {}", reference.sha256),
            )?;
            total += reference.size;
            next = checkpoint.previous.clone();
            held.push(Held {
                reference,
                checkpoint,
                raw,
                job,
            });
        }
        let first = &held.last().unwrap().checkpoint;
        let root = self.baseline(&first.root, product)?;
        let marker = root.origin.record.marker();
        let commit = &root.materials.world.source.commit;
        Wire::require(
            first
                .markers
                .iter()
                .any(|tag| tag.marker == marker && &tag.commit == commit),
            "retained history omitted or retargeted its adopted stable",
        )?;
        for entry in &held {
            let reader = producer::Reader(&self.0);
            reader.run(&entry.checkpoint.native)?;
            Wire::require(
                reader.role(
                    &entry.checkpoint.native,
                    "[retain] semantic history",
                    &format!("[retain] history {}", entry.reference.sha256),
                )? == entry.job
                    && entry.reference.bytes(&self.0, product)? == entry.raw,
                "retained checkpoint or native producer moved during observation",
            )?;
        }
        Wire::require(
            policy::Reader(&self.0).current()? == authority
                && Wire::repository(&self.0, &format!("PerishLab/{product}"))? == repository,
            "history authority moved during observation",
        )?;
        Ok(History {
            reference: reference.clone(),
            checkpoints: held.into_iter().map(|entry| entry.checkpoint).collect(),
            root,
            authority,
            authentication: true,
            prior: "unknown",
            discovery: None,
            publication: false,
        })
    }
}
