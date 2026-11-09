use super::{Fault, Inspector, Read, Reference, Selection, Wire, producer};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct Choice {
    reference: Reference,
    record: Selection,
    producer: Value,
    job: Value,
    authentication: bool,
    publication: bool,
}

impl Choice {
    pub fn reference(&self) -> &Reference {
        &self.reference
    }
    pub fn record(&self) -> &Selection {
        &self.record
    }
}

impl<R: Read> Inspector<R> {
    pub fn choice(&self, reference: &Reference, product: &str) -> Result<Choice, Fault> {
        let authority = self.current()?;
        let raw = reference.bytes(&self.0, product)?;
        let record = Selection::parse(&raw)?;
        Wire::require(record.product == product, "selection names another product")?;
        let reader = producer::Reader(&self.0);
        reader.lineage(&record.native.commit, &authority.source.commit)?;
        let producer = reader.run(&record.native)?;
        let confirmation = format!("[select] family {}", reference.sha256);
        let job = reader.role(&record.native, "[select] stable family", &confirmation)?;
        Wire::require(
            reference.bytes(&self.0, product)? == raw
                && reader.run(&record.native)? == producer
                && reader.role(&record.native, "[select] stable family", &confirmation)? == job
                && self.current()? == authority,
            "selection bytes, issuer or authority moved",
        )?;
        Ok(Choice {
            reference: reference.clone(),
            record,
            producer,
            job,
            authentication: true,
            publication: false,
        })
    }
}
