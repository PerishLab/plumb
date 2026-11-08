mod capture;
mod discovery;
mod reader;
mod record;

pub use capture::Retention;
pub use discovery::Discovery;
pub use record::Checkpoint;

use super::{Authority, Baseline, Reference};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct History {
    reference: Reference,
    checkpoints: Vec<Checkpoint>,
    root: Baseline,
    authority: Authority,
    authentication: bool,
    prior: &'static str,
    discovery: Option<String>,
    publication: bool,
}

impl History {
    pub const CHECKPOINTS: usize = 128;
    pub const BYTES: u64 = 16 * 1024 * 1024;

    pub fn reference(&self) -> &Reference {
        &self.reference
    }

    pub fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }

    pub fn root(&self) -> &Baseline {
        &self.root
    }
}
