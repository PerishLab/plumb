#[path = "../adoption/mod.rs"]
mod baseline;
#[path = "../semantics/comparison.rs"]
mod comparison;
#[path = "../semantics/declaration.rs"]
mod declaration;
#[path = "../history/mod.rs"]
mod history;
mod native;
mod observation;
mod policy;
#[path = "../preparation/mod.rs"]
mod preparation;
mod producer;
mod record;
mod reference;
mod review;
mod wire;
mod world;

use crate::windtun::qualification::{Fault, Read};
pub use baseline::{Baseline, Inventory, Materials, Tag};
pub use comparison::{Binding, Comparison, Projection};
pub use declaration::{Definition, Judgment};
pub use history::{
    Checkpoint, Choice, Discovery, Family, History, Nomination, Retention, Selected, Selection,
    Selector, Transition, Validation,
};
pub use native::Native;
pub use policy::{Authority, Requirements, Scope};
pub use preparation::{File, Frame, Payload, Preparation, Prepared};
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

    fn observed<T>(
        &self,
        inspect: impl FnOnce(&Inspector<observation::Observation<'_>>) -> Result<T, Fault>,
    ) -> Result<T, Fault> {
        let held = Inspector::new(observation::Observation::new(&self.0));
        let result = inspect(&held)?;
        held.0.confirm()?;
        Ok(result)
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

    pub fn comparison(&self, location: &str, product: &str) -> Result<Review<Comparison>, Fault> {
        review::Reader(&self.0).comparison(location, product)
    }
}
