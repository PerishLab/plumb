mod publication;

use super::{
    Adoption, Definition, Inspector, Origin, Record, Reference, Review, World, policy, producer,
    wire::Wire,
};
use crate::semantics::collection::{Collector, Report};
use crate::windtun::qualification::{Fault, Read, State};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct Baseline {
    pub origin: Origin,
    pub materials: Materials,
}

#[derive(Debug, Serialize)]
pub struct Materials {
    pub world: World,
    pub collection: Report,
    pub review: Review<Definition>,
    pub published: Value,
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

impl<R: Read> Inspector<R> {
    pub fn baseline(&self, reference: &Reference, product: &str) -> Result<Baseline, Fault> {
        let origin = producer::Reader(&self.0).observe(reference, product)?;
        let Record::Adoption(adoption) = &origin.record else {
            return Err(Fault::refused(
                "initial baseline requires an authenticated adoption record",
            ));
        };
        let materials = self.materials(adoption)?;
        Wire::require(
            policy::Reader(&self.0).current()? == origin.authority,
            "adoption issuer authority moved during material verification",
        )?;
        Ok(Baseline { origin, materials })
    }

    pub fn materials(&self, adoption: &Adoption) -> Result<Materials, Fault> {
        Record::Adoption(Box::new(adoption.clone())).validate()?;
        let product = &adoption.product;
        let authority = policy::Reader(&self.0).current()?;
        let world = World::parse(&adoption.world.bytes(&self.0, product)?)?;
        Wire::require(
            world.source.source() == adoption.release.snapshot.source
                && world.source.marker == adoption.release.marker,
            "adopted world names another formal source",
        )?;
        if world.tree != world.source.tree {
            return Err(Fault::unknown(
                "effective source differs from the archived source; complete effective-source materials are unavailable",
            ));
        }
        let publication = publication::Reader(&self.0).observe(&world)?;
        let source = &adoption.release.snapshot.source;
        let collection = Collector::new(Access(&self.0)).historical(
            &source.repository,
            &source.commit,
            &adoption.scenes,
        );
        Self::collected(&collection)?;
        let snapshot = collection.snapshot.as_ref().unwrap();
        let digest = adoption.release.snapshot.digest().map_err(Fault::refused)?;
        Wire::require(
            snapshot.digest().map_err(Fault::refused)? == digest,
            "adopted snapshot differs from exact source and formal Scene materials",
        )?;
        let review = self.adoption(&adoption.review.url, product)?;
        Wire::require(
            review.reference == adoption.review,
            "adoption review bytes differ from its bound reference",
        )?;
        Wire::require(
            review.judgment.marker == adoption.release.marker
                && review.judgment.snapshot == digest
                && review.judgment.world == adoption.world.sha256,
            "adoption review does not bind the exact release, boundary and original world",
        )?;
        Wire::require(
            publication::Reader(&self.0).observe(&world)? == publication
                && policy::Reader(&self.0).current()? == authority,
            "adopted publication or authority moved during observation",
        )?;
        Ok(Materials {
            world,
            collection,
            review,
            published: publication,
        })
    }

    fn collected(report: &Report) -> Result<(), Fault> {
        if report.state == "observed" && report.snapshot.is_some() {
            return Ok(());
        }
        Err(Fault {
            state: if report.state == "refused" {
                State::Refused
            } else {
                State::Unknown
            },
            reason: report
                .reason
                .clone()
                .unwrap_or_else(|| "adopted material closure is incomplete".into()),
        })
    }
}
