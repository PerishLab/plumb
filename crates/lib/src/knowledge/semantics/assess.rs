use serde::Serialize;
use std::collections::BTreeMap;

use super::{Decision, Material, Request, Snapshot, version::Markers};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Same,
    Changed,
    Unknown,
    Conflict,
    Human,
}

#[derive(Debug, Serialize)]
pub struct Changes {
    pub source: bool,
    pub coverage: bool,
    pub added: Vec<Material>,
    pub removed: Vec<Material>,
    pub changed: Vec<Material>,
}

impl Changes {
    fn between(before: &Snapshot, after: &Snapshot) -> Self {
        let old: BTreeMap<_, _> = before
            .materials
            .iter()
            .map(|item| ((item.kind, &item.name), item))
            .collect();
        let new: BTreeMap<_, _> = after
            .materials
            .iter()
            .map(|item| ((item.kind, &item.name), item))
            .collect();
        let added = new
            .iter()
            .filter(|(key, _)| !old.contains_key(*key))
            .map(|(_, item)| (*item).clone())
            .collect();
        let removed = old
            .iter()
            .filter(|(key, _)| !new.contains_key(*key))
            .map(|(_, item)| (*item).clone())
            .collect();
        let changed = new
            .iter()
            .filter(|(key, item)| old.get(*key).is_some_and(|old| old != *item))
            .map(|(_, item)| (*item).clone())
            .collect();
        Self {
            source: before.source != after.source,
            coverage: before.scope() != after.scope(),
            added,
            removed,
            changed,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Assessment {
    pub schema: &'static str,
    pub product: String,
    pub state: State,
    pub marker: Option<String>,
    pub baseline: String,
    pub family: String,
    pub anchor: String,
    pub before: String,
    pub after: String,
    pub changes: Changes,
    pub review: Option<String>,
    pub major: Option<String>,
    pub authentication: bool,
    pub publication: bool,
    pub reason: Option<String>,
}

impl Assessment {
    pub(super) fn new(request: &Request) -> Result<Self, String> {
        let stable = &request.baseline.stable;
        let before = stable.snapshot.digest()?;
        let after = request.candidate.digest()?;
        let mut held = Self {
            schema: "plumb.release.assessment/v1",
            product: request.candidate.product.clone(),
            state: State::Unknown,
            marker: None,
            baseline: stable.marker.clone(),
            family: request.baseline.anchor.marker.clone(),
            anchor: request.baseline.anchor.snapshot.digest()?,
            before,
            after,
            changes: Changes::between(&stable.snapshot, &request.candidate),
            review: request
                .review
                .as_ref()
                .map(|review| review.reference.clone()),
            major: request.major.clone(),
            authentication: false,
            publication: false,
            reason: None,
        };
        held.decide(request)?;
        Ok(held)
    }

    fn decide(&mut self, request: &Request) -> Result<(), String> {
        let markers = Markers(&request.reservations);
        markers.highest(&self.baseline)?;
        if request.major.is_some() {
            self.state = State::Human;
            self.reason = Some("X requires independent verification of the user's decision".into());
            return Ok(());
        }
        let decision = self.decision(request)?;
        let Some(decision) = decision else {
            self.reason = Some("changed inputs require an exact boundary review".into());
            return Ok(());
        };
        self.marker = markers.next(&self.baseline, decision)?;
        self.state = match (decision, self.marker.is_some()) {
            (_, false) => State::Conflict,
            (Decision::Same, true) => State::Same,
            (Decision::Changed, true) => State::Changed,
        };
        if self.state == State::Conflict {
            self.reason = Some("reserved markers prevent a version in the required semantic dimension; retain ordering and seek disposition".into());
        }
        Ok(())
    }

    fn decision(&self, request: &Request) -> Result<Option<Decision>, String> {
        if self.changes.coverage {
            if request
                .review
                .as_ref()
                .is_some_and(|review| review.decision == Decision::Same)
            {
                return Err("a review cannot preserve changed declared coverage".into());
            }
            return Ok(Some(Decision::Changed));
        }
        if self.before == self.after {
            return Ok(Some(Decision::Same));
        }
        Ok(request.review.as_ref().map(|review| review.decision))
    }

    pub fn code(&self) -> i32 {
        match self.state {
            State::Same | State::Changed => 0,
            State::Conflict => 1,
            State::Unknown | State::Human => 3,
        }
    }
}
