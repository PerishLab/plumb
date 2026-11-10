use super::super::{Inspector, Reference, Transition, wire::Wire};
use super::Family;
use crate::semantics::{
    Assessment, Baseline, Decision, Release, Request, Review, State, version::Markers,
};
use crate::windtun::qualification::{Fault, Read};

impl<R: Read> Inspector<R> {
    pub(super) fn advance(
        &self,
        family: &mut Family,
        supplied: (&Reference, &Transition),
        product: &str,
    ) -> Result<(), Fault> {
        let (reference, transition) = supplied;
        let intent = &transition.intent;
        Wire::require(
            intent.root == family.root
                && intent.previous == family.reference
                && intent.anchor == family.anchor,
            "transition replaced its root, predecessor or original family anchor",
        )?;
        if intent.direct.is_some() || intent.request.major.is_some() {
            return Err(Fault::unknown(
                "X transition requires independent verification of the user's decision",
            ));
        }
        let candidate = self.formal(
            (&intent.world, transition.source.as_ref()),
            &intent.scenes,
            product,
        )?;
        Wire::require(
            candidate.world().source.marker == intent.marker,
            "transition names another formal marker",
        )?;
        let history = self.history(&transition.history, product)?;
        Wire::require(
            history.root().origin.reference == family.root,
            "transition history replaced its original adoption",
        )?;
        Wire::require(
            family.history.as_ref().is_none_or(|previous| {
                history.reference() == previous
                    || history
                        .checkpoints()
                        .iter()
                        .any(|item| item.previous.as_ref() == Some(previous))
            }),
            "transition discarded its previously retained history",
        )?;
        Wire::require(
            history.checkpoints()[0].markers.iter().any(|tag| {
                tag.marker == family.stable.world().source.marker
                    && tag.commit == family.stable.snapshot().source.commit
            }),
            "transition history omitted or retargeted its preceding stable",
        )?;
        let observation = Wire::sha(&Wire::canonical(&(&history, &candidate))?);
        let mut reservations = Vec::new();
        for tag in &history.checkpoints()[0].markers {
            if tag.marker == intent.marker {
                Wire::require(
                    tag.commit == candidate.snapshot().source.commit,
                    "transition retargets a retained marker",
                )?;
            } else {
                reservations.push(tag.marker.clone());
            }
        }
        let reviewref = intent
            .review
            .as_ref()
            .ok_or_else(|| Fault::unknown("transition has no exact world-bound review"))?;
        let review = self.comparison(&reviewref.url, product)?;
        Wire::require(
            review.reference == *reviewref,
            "transition review bytes differ",
        )?;
        let judgment = Wire::sha(&Wire::canonical(&review)?);
        let projection = review.judgment.frames(
            &reviewref.url,
            [&family.stable, &candidate, &family.original],
        )?;
        let continuity = family
            .continuity
            .as_ref()
            .map(|location| {
                Ok::<_, Fault>(Review {
                    before: projection.anchor.digest().map_err(Fault::refused)?,
                    after: projection.before.digest().map_err(Fault::refused)?,
                    anchor: projection.anchor.digest().map_err(Fault::refused)?,
                    decision: Decision::Same,
                    reference: location.clone(),
                })
            })
            .transpose()?;
        let request = Request {
            schema: Request::SCHEMA.into(),
            baseline: Baseline {
                stable: Release {
                    marker: family.stable.world().source.marker.clone(),
                    snapshot: projection.before,
                },
                anchor: Release {
                    marker: family.original.world().source.marker.clone(),
                    snapshot: projection.anchor,
                },
                review: continuity,
            },
            candidate: projection.after,
            review: Some(projection.review),
            reservations,
            major: None,
        };
        Wire::require(
            request == intent.request,
            "supplied transition judgment differs from independently derived inputs",
        )?;
        let assessed = family.judge(&request, &intent.marker)?;
        Wire::require(
            matches!(assessed.state, State::Same | State::Changed)
                && assessed.marker.as_deref() == Some(&intent.marker),
            "transition marker disagrees with the shared semantic judgment",
        )?;
        if assessed.state == State::Changed {
            family.anchor = reference.clone();
            family.original = candidate.clone();
            family.continuity = None;
        } else {
            family.continuity = Some(reviewref.url.clone());
        }
        family.history = Some(transition.history.clone());
        family.observations.extend([observation, judgment]);
        family.reference = reference.clone();
        family.stable = candidate;
        family.records.push(reference.clone());
        Ok(())
    }
}

impl Family {
    fn judge(&self, request: &Request, marker: &str) -> Result<Assessment, Fault> {
        request.validate().map_err(Fault::refused)?;
        let target = Markers::stable(marker).map_err(Fault::refused)?;
        let baseline =
            Markers::stable(&self.stable.world().source.marker).map_err(Fault::refused)?;
        if request.major.is_some() || target.major != baseline.major {
            return Err(Fault::unknown(
                "X transition requires independent verification of the user's decision",
            ));
        }
        if target <= baseline {
            return Err(Fault::refused(
                "historical stable does not advance its predecessor",
            ));
        }
        let mut projected = request.clone();
        projected.reservations.clear();
        for marker in &request.reservations {
            let version = Markers::parse(marker).map_err(Fault::refused)?;
            if version <= target {
                projected.reservations.push(marker.clone());
            }
        }
        projected.assess().map_err(Fault::refused)
    }
}
