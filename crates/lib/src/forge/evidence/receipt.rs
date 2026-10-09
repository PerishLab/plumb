use super::super::operation::{Operation, Outcome, Request};
use super::{Reference, Verified};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    reference: Reference,
    operation: Operation,
}

impl Verified {
    pub fn settle(&self, request: &Request) -> Result<Receipt, String> {
        let operation = self
            .record()
            .operation()
            .ok_or("lane completion needs operation evidence")?;
        if operation.request() != request {
            return Err("lane completion evidence belongs to another exact request".into());
        }
        if !matches!(operation.outcome(), Outcome::Applied { .. }) {
            return Err("lane completion needs an applied observation".into());
        }
        Ok(Receipt {
            reference: self.reference().clone(),
            operation: operation.clone(),
        })
    }
}

impl Receipt {
    pub fn reference(&self) -> &Reference {
        &self.reference
    }

    pub fn operation(&self) -> &Operation {
        &self.operation
    }
}
