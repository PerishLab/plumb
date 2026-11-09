use super::super::{Frame, Inspector, Origin, Record, Reference, wire::Wire};
use crate::windtun::qualification::{Fault, Read};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize)]
pub struct Family {
    pub(super) reference: Reference,
    pub(super) root: Reference,
    pub(super) stable: Frame,
    pub(super) anchor: Reference,
    pub(super) original: Frame,
    pub(super) records: Vec<Reference>,
    pub(super) continuity: Option<String>,
    pub(super) history: Option<Reference>,
    pub(super) observations: Vec<String>,
    prior: &'static str,
    pub(super) authentication: bool,
    publication: bool,
}

impl Family {
    pub const RECORDS: usize = 64;
    pub const BYTES: u64 = 8 * 1024 * 1024;

    fn initial(reference: &Reference, root: Frame) -> Self {
        Self {
            reference: reference.clone(),
            root: reference.clone(),
            stable: root.clone(),
            anchor: reference.clone(),
            original: root,
            records: vec![reference.clone()],
            continuity: None,
            history: None,
            observations: Vec::new(),
            prior: "unknown",
            authentication: true,
            publication: false,
        }
    }

    pub fn reference(&self) -> &Reference {
        &self.reference
    }

    pub fn root(&self) -> &Reference {
        &self.root
    }

    pub fn stable(&self) -> &Frame {
        &self.stable
    }

    pub fn anchor(&self) -> &Reference {
        &self.anchor
    }

    pub fn original(&self) -> &Frame {
        &self.original
    }
}

impl<R: Read> Inspector<R> {
    pub fn family(&self, reference: &Reference, product: &str) -> Result<Family, Fault> {
        let authority = self.current()?;
        let mut records: Vec<Origin> = Vec::new();
        let mut seen = BTreeSet::new();
        let mut next = reference.clone();
        let mut bytes = 0_u64;
        loop {
            if records.len() >= Family::RECORDS || bytes.saturating_add(next.size) > Family::BYTES {
                return Err(Fault::unknown(
                    "family history exceeds complete observation bounds",
                ));
            }
            Wire::require(seen.insert(next.sha256.clone()), "family repeats a record")?;
            let origin = self.record(&next, product)?;
            bytes += next.size;
            let previous = match &origin.record {
                Record::Adoption(_) => None,
                Record::Transition(held) => Some(held.intent.previous.clone()),
                _ => {
                    return Err(Fault::refused(
                        "family needs adoption or retained transition evidence",
                    ));
                }
            };
            records.push(origin);
            let Some(previous) = previous else { break };
            next = previous;
        }
        let first = &records.last().unwrap().reference;
        let root = self.root(first, product)?;
        let mut family = Family::initial(first, root.clone());
        for origin in records.iter().rev().skip(1) {
            let Record::Transition(transition) = &origin.record else {
                unreachable!()
            };
            self.advance(&mut family, (&origin.reference, transition), product)?;
        }
        let refreshed = self.root(first, product)?;
        Wire::require(
            Wire::canonical(&root)? == Wire::canonical(&refreshed)?,
            "family root materials moved during observation",
        )?;
        let mut repeated = Family::initial(first, refreshed);
        for origin in records.iter().rev().skip(1) {
            let Record::Transition(transition) = &origin.record else {
                unreachable!()
            };
            self.advance(&mut repeated, (&origin.reference, transition), product)?;
        }
        Wire::require(
            Wire::canonical(&family)? == Wire::canonical(&repeated)?,
            "family materials, history or review moved during observation",
        )?;
        for origin in &records {
            Wire::require(
                Wire::canonical(&self.record(&origin.reference, product)?)?
                    == Wire::canonical(origin)?,
                "family issuer or evidence moved during observation",
            )?;
        }
        Wire::require(
            self.current()? == authority,
            "family authority moved during observation",
        )?;
        Ok(family)
    }
}
