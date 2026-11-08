#[path = "../adoption/mod.rs"]
mod baseline;
#[path = "../semantics/declaration.rs"]
mod declaration;
mod native;
mod policy;
mod producer;
mod record;
mod reference;
mod review;
mod wire;
mod world;

use crate::windtun::qualification::{Fault, Read};
pub use baseline::{Baseline, Materials};
pub use declaration::{Definition, Judgment};
pub use native::Native;
pub use policy::{Authority, Requirements, Scope};
pub use producer::Origin;
pub use record::{Adoption, Canary, Intent, Kind, Publication, Record, Trial};
pub use reference::Reference;
pub use review::{Author, Review};
pub use world::{Control, Controls, Point, Stable, World};

pub struct Inspector<R>(R);

impl<R: Read> Inspector<R> {
    pub fn new(reader: R) -> Self {
        Self(reader)
    }

    pub fn current(&self) -> Result<Authority, Fault> {
        policy::Reader(&self.0).current()
    }

    pub fn record(&self, reference: &Reference, product: &str) -> Result<Origin, Fault> {
        producer::Reader(&self.0).observe(reference, product)
    }

    pub fn review(&self, location: &str, product: &str) -> Result<Review, Fault> {
        review::Reader(&self.0).inspect(location, product)
    }

    pub fn adoption(&self, location: &str, product: &str) -> Result<Review<Definition>, Fault> {
        review::Reader(&self.0).adoption(location, product)
    }
}
