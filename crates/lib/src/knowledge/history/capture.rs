use super::super::{Inspector, Native, Reference, Tag, wire::Wire};
use super::{Checkpoint, Discovery, History};
use crate::windtun::qualification::{Fault, Read};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Retention {
    pub schema: String,
    pub root: Reference,
    pub previous: Option<Reference>,
    pub native: Native,
}

impl Retention {
    pub const SCHEMA: &str = "wharf.release.semantics.history.request/v1";

    pub fn parse(raw: &[u8]) -> Result<Self, Fault> {
        let held: Self = Wire::parse(raw)?;
        Wire::require(
            held.schema == Self::SCHEMA && Wire::canonical(&held)? == raw,
            "history request schema or canonical bytes differ",
        )?;
        Ok(held)
    }
}

impl<R: Read> Inspector<R> {
    pub fn retain(&self, request: &Retention, product: &str) -> Result<Checkpoint, Fault> {
        Wire::require(
            request.schema == Retention::SCHEMA,
            "unsupported history request",
        )?;
        request.native.validate()?;
        request.root.validate(product)?;
        if let Some(reference) = &request.previous {
            reference.validate(product)?;
        }
        let authority = self.current()?;
        let discovered = Discovery::read(&self.0, product)?;
        if discovered.is_none() && request.previous.is_some() {
            return Err(Fault::unknown("current retained history is undiscovered"));
        }
        Wire::require(
            discovered.as_ref().map(|head| &head.reference) == request.previous.as_ref(),
            "history request does not bind current discovery",
        )?;
        let previous = request
            .previous
            .as_ref()
            .map(|_| self.discover(product))
            .transpose()?;
        let initial = if previous.is_none() {
            Some(self.baseline(&request.root, product)?)
        } else {
            None
        };
        if previous
            .as_ref()
            .is_some_and(|history| history.checkpoints().len() >= History::CHECKPOINTS)
        {
            return Err(Fault::unknown("history observation capacity is exhausted"));
        }
        let root = match &previous {
            Some(history) => {
                Wire::require(
                    history.root().origin.reference == request.root,
                    "history cannot replace its original adoption",
                )?;
                history.root()
            }
            None => initial.as_ref().unwrap(),
        };
        let inventory = self.markers(product)?;
        let mut markers = BTreeMap::new();
        let mut sequence = 1;
        if let Some(history) = &previous {
            let parent = &history.checkpoints()[0];
            Wire::require(
                parent.repository == inventory.repository,
                "history repository authority changed",
            )?;
            sequence = parent
                .sequence
                .checked_add(1)
                .ok_or_else(|| Fault::unknown("history sequence exhausted"))?;
            for tag in &parent.markers {
                markers.insert(tag.marker.clone(), tag.commit.clone());
            }
        }
        Wire::require(
            inventory.markers.iter().any(|tag| {
                tag.marker == root.origin.record.marker()
                    && tag.commit == root.materials.world.source.commit
            }),
            "current inventory omitted or retargeted the adopted stable",
        )?;
        for tag in inventory.markers {
            Wire::require(
                markers
                    .get(&tag.marker)
                    .is_none_or(|commit| commit == &tag.commit),
                "current tag retargets a retained reservation",
            )?;
            markers.insert(tag.marker, tag.commit);
        }
        let checkpoint = Checkpoint {
            schema: Checkpoint::SCHEMA.into(),
            product: product.into(),
            repository: inventory.repository,
            root: request.root.clone(),
            previous: request.previous.clone(),
            sequence,
            markers: markers
                .into_iter()
                .map(|(marker, commit)| Tag { marker, commit })
                .collect(),
            native: request.native.clone(),
        };
        checkpoint.validate()?;
        let mut total = checkpoint.encode()?.len() as u64;
        if let Some(history) = &previous {
            for parent in history.checkpoints() {
                total += parent.encode()?.len() as u64;
            }
        }
        if total > History::BYTES {
            return Err(Fault::unknown(
                "next history would exceed complete observation bounds",
            ));
        }
        if let Some(history) = &previous {
            checkpoint.follows(history.reference(), &history.checkpoints()[0])?;
        }
        Wire::require(
            self.current()? == authority && Discovery::read(&self.0, product)? == discovered,
            "history authority or discovery moved during preparation",
        )?;
        Ok(checkpoint)
    }
}
