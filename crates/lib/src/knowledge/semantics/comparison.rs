use super::{World, wire::Wire};
use crate::semantics::{Decision, Kind, Material, Review, Snapshot};
use crate::windtun::qualification::Fault;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub snapshot: String,
    pub world: String,
}

impl Binding {
    pub fn new(snapshot: &Snapshot, world: &World) -> Result<Self, Fault> {
        snapshot.validate().map_err(Fault::refused)?;
        world.validate()?;
        Wire::require(
            snapshot.source == world.source.source(),
            "review world names another snapshot source",
        )?;
        if world.tree != world.source.tree {
            return Err(Fault::unknown(
                "complete effective-source materials are unavailable for boundary review",
            ));
        }
        Ok(Self {
            snapshot: snapshot.digest().map_err(Fault::refused)?,
            world: Wire::sha(&Wire::canonical(world)?),
        })
    }

    fn project(snapshot: &Snapshot, world: &World) -> Result<(Self, Snapshot), Fault> {
        let binding = Self::new(snapshot, world)?;
        let name = "plumb/release/world.json";
        Wire::require(
            !snapshot
                .materials
                .iter()
                .any(|item| item.kind == Kind::Runtime && item.name == name),
            "source snapshot already contains the reserved review world material",
        )?;
        let mut projected = snapshot.clone();
        projected.materials.push(
            Material::new(Kind::Runtime, name.into(), &Wire::canonical(world)?, false)
                .map_err(Fault::refused)?,
        );
        projected.validate().map_err(Fault::refused)?;
        Ok((binding, projected))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Comparison {
    pub schema: String,
    pub product: String,
    pub before: Binding,
    pub after: Binding,
    pub anchor: Binding,
    pub decision: Decision,
    pub basis: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Projection {
    pub before: Snapshot,
    pub after: Snapshot,
    pub anchor: Snapshot,
    pub review: Review,
}

impl Comparison {
    pub const SCHEMA: &str = "plumb.release.review/v2";

    pub fn parse(body: &str) -> Result<Self, Fault> {
        Wire::require(
            body.matches("<!-- plumb.release.review/").count() == 1,
            "world-bound review requires one unambiguous declaration",
        )?;
        let held: Self = Wire::marked(body, Self::SCHEMA)?;
        held.validate()?;
        Ok(held)
    }

    fn validate(&self) -> Result<(), Fault> {
        Wire::require(
            self.schema == Self::SCHEMA,
            "unsupported world-bound review schema",
        )?;
        Wire::name(&self.product)?;
        for binding in [&self.before, &self.after, &self.anchor] {
            Wire::hex(&binding.snapshot, 64)?;
            Wire::hex(&binding.world, 64)?;
        }
        Wire::require(
            !self.basis.trim().is_empty() && self.basis.len() <= 16 * 1024,
            "world-bound review requires its bounded actual basis",
        )?;
        Wire::require(
            self.before != self.after || self.decision == Decision::Same,
            "identical boundary and world inputs cannot claim a change",
        )
    }

    pub fn project(
        &self,
        reference: &str,
        inputs: [(&Snapshot, &World); 3],
    ) -> Result<Projection, Fault> {
        self.validate()?;
        super::super::wire::Wire::reference(reference).map_err(Fault::refused)?;
        let [(before, old), (after, current), (anchor, original)] = inputs;
        for snapshot in [before, after, anchor] {
            Wire::require(
                snapshot.product == self.product,
                "world-bound review names another product",
            )?;
        }
        let (old, before) = Binding::project(before, old)?;
        let (current, after) = Binding::project(after, current)?;
        let (original, anchor) = Binding::project(anchor, original)?;
        Wire::require(
            old == self.before && current == self.after && original == self.anchor,
            "review does not bind all three exact snapshot and world pairs",
        )?;
        let review = Review {
            before: before.digest().map_err(Fault::refused)?,
            after: after.digest().map_err(Fault::refused)?,
            anchor: anchor.digest().map_err(Fault::refused)?,
            decision: self.decision,
            reference: reference.into(),
        };
        review
            .bind(&before, &after, &anchor)
            .map_err(Fault::refused)?;
        Ok(Projection {
            before,
            after,
            anchor,
            review,
        })
    }
}
