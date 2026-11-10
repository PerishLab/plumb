use super::super::{Inspector, Record, Reference, Transition, wire::Wire};
use super::Family;
use crate::semantics::State;
use crate::semantics::version::Markers;
use crate::windtun::qualification::{Fault, Read};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Validation {
    schema: &'static str,
    freshness: &'static str,
    previous: Family,
    derived: Family,
    authentication: bool,
    publication: bool,
}

impl<R: Read> Inspector<R> {
    pub fn transition(&self, transition: &Transition, product: &str) -> Result<Validation, Fault> {
        self.observed(|held| held.validate(transition, product))
    }

    fn validate(&self, transition: &Transition, product: &str) -> Result<Validation, Fault> {
        let authority = self.current()?;
        let inventory = self.markers(product)?;
        let mut latest = None;
        for tag in &inventory.markers {
            let version = Markers::parse(&tag.marker).map_err(Fault::refused)?;
            if version.pre.is_empty() && latest.as_ref().is_none_or(|held| &version > held) {
                latest = Some(version);
            }
        }
        Wire::require(
            latest.map(|version| format!("v{version}")).as_deref()
                == Some(&transition.intent.marker),
            "transition does not name current latest stable",
        )?;
        let history = self.discover(product)?;
        Wire::require(
            history.reference() == &transition.history,
            "transition does not bind current retained history",
        )?;
        if !inventory.markers.iter().all(|tag| {
            history.checkpoints()[0]
                .markers
                .iter()
                .any(|held| held == tag)
        }) {
            return Err(Fault::unknown(
                "current retained history does not cover current tag inventory",
            ));
        }
        let mut validation = self.historical(transition, product)?;
        let assessed = transition.intent.request.assess().map_err(Fault::refused)?;
        Wire::require(
            matches!(assessed.state, State::Same | State::Changed)
                && assessed.marker.as_deref() == Some(&transition.intent.marker),
            "current transition marker disagrees with the complete reservation judgment",
        )?;
        Wire::require(
            self.markers(product)? == inventory
                && Wire::canonical(&self.discover(product)?)? == Wire::canonical(&history)?
                && self.current()? == authority,
            "transition inventory, history or authority moved",
        )?;
        validation.freshness = "current";
        Ok(validation)
    }

    pub fn historical(&self, transition: &Transition, product: &str) -> Result<Validation, Fault> {
        self.observed(|held| held.derive(transition, product))
    }

    fn derive(&self, transition: &Transition, product: &str) -> Result<Validation, Fault> {
        let record = Record::Transition(Box::new(transition.clone()));
        let raw = record.encode()?;
        Wire::require(
            record.product() == product,
            "transition names another product",
        )?;
        let authority = self.current()?;
        let previous = self.family(&transition.intent.previous, product)?;
        if previous.records.len() >= Family::RECORDS
            || previous
                .records
                .iter()
                .map(|reference| reference.size)
                .sum::<u64>()
                .saturating_add(raw.len() as u64)
                > Family::BYTES
        {
            return Err(Fault::unknown(
                "next transition exceeds complete family observation bounds",
            ));
        }
        let sha256 = Wire::sha(&raw);
        let reference = Reference {
            url: format!("https://releases.{product}.perish.uk/v1/semantics/objects/{sha256}.json"),
            sha256,
            size: raw.len() as u64,
        };
        let mut derived = previous.clone();
        self.advance(&mut derived, (&reference, transition), product)?;
        derived.authentication = false;
        let refreshed = self.family(&transition.intent.previous, product)?;
        Wire::require(
            Wire::canonical(&previous)? == Wire::canonical(&refreshed)?,
            "transition preceding family moved",
        )?;
        let mut repeated = refreshed;
        self.advance(&mut repeated, (&reference, transition), product)?;
        repeated.authentication = false;
        Wire::require(
            Wire::canonical(&derived)? == Wire::canonical(&repeated)?,
            "transition materials or review moved",
        )?;
        Wire::require(
            self.current()? == authority,
            "historical transition authority moved",
        )?;
        Ok(Validation {
            schema: "plumb.release.transition.materials/v1",
            freshness: "historical",
            previous,
            derived,
            authentication: false,
            publication: false,
        })
    }
}
