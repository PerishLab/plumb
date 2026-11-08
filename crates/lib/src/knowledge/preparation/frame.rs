use super::super::{
    Binding, Comparison, Inspector, Preparation, Projection, Reference, World, wire::Wire,
};
use crate::seat::depot::v3::Pointer;
use crate::semantics::{Kind, Material, Snapshot, collection::Collector};
use crate::windtun::qualification::{Fault, Read, State};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Frame {
    snapshot: Snapshot,
    world: World,
}

struct Access<'a, R>(&'a R);

impl<R: Read> Read for Access<'_, R> {
    fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>, Fault> {
        self.0.bytes(url, limit)
    }

    fn optional(&self, url: &str, limit: usize) -> Result<Option<Vec<u8>>, Fault> {
        self.0.optional(url, limit)
    }

    fn image(&self, reference: &str) -> Result<String, Fault> {
        self.0.image(reference)
    }
}

impl Frame {
    pub const PREPARATION: &str = "plumb/release/preparation.json";

    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn binding(&self) -> Result<Binding, Fault> {
        Binding::observed(&self.snapshot, &self.world)
    }

    fn project(&self) -> Result<(Binding, Snapshot), Fault> {
        Binding::augmented(&self.snapshot, &self.world, self.binding()?)
    }
}

impl Comparison {
    pub fn frames(&self, reference: &str, inputs: [&Frame; 3]) -> Result<Projection, Fault> {
        let [before, after, anchor] = inputs;
        self.projected(
            reference,
            [before.project()?, after.project()?, anchor.project()?],
        )
    }
}

impl<R: Read> Inspector<R> {
    pub fn root(&self, reference: &Reference, product: &str) -> Result<Frame, Fault> {
        let baseline = self.baseline(reference, product)?;
        let snapshot = baseline.materials.collection.snapshot.ok_or_else(|| {
            Fault::unknown("authenticated root has no complete boundary collection")
        })?;
        Ok(Frame {
            snapshot,
            world: baseline.materials.world,
        })
    }

    pub fn frame(
        &self,
        reference: &Reference,
        scenes: &Pointer,
        product: &str,
    ) -> Result<Frame, Fault> {
        let raw = reference.bytes(&self.0, product)?;
        let descriptor = Preparation::parse(&raw)?;
        let prepared = self.prepared(&descriptor, product)?;
        let report = Collector::new(Access(&self.0)).historical(
            &prepared.world.source.repository,
            &prepared.world.source.commit,
            scenes,
        );
        if report.state != "observed" || report.snapshot.is_none() {
            return Err(Fault {
                state: if report.state == "refused" {
                    State::Refused
                } else {
                    State::Unknown
                },
                reason: report
                    .reason
                    .unwrap_or_else(|| "prepared boundary collection is incomplete".into()),
            });
        }
        let mut snapshot = report.snapshot.unwrap();
        Wire::require(
            snapshot.product == product && snapshot.source == prepared.world.source.source(),
            "prepared frame names another collected source",
        )?;
        Wire::require(
            !snapshot
                .materials
                .iter()
                .any(|item| item.kind == Kind::Runtime && item.name == Frame::PREPARATION),
            "collection already contains the reserved preparation material",
        )?;
        snapshot.materials.push(
            Material::new(Kind::Runtime, Frame::PREPARATION.into(), &raw, false)
                .map_err(Fault::refused)?,
        );
        snapshot.validate().map_err(Fault::refused)?;
        Wire::require(
            reference.bytes(&self.0, product)? == raw && self.current()? == prepared.authority,
            "prepared frame descriptor or authority moved during collection",
        )?;
        Ok(Frame {
            snapshot,
            world: prepared.world,
        })
    }
}
