mod assess;
#[cfg(feature = "windtun")]
#[path = "../collection/mod.rs"]
pub mod collection;
mod identity;
mod material;
mod review;
pub mod subjects;
mod version;
mod wire;

pub use assess::{Assessment, Changes, State};
pub use identity::{Baseline, Release, Snapshot, Source};
pub use material::{Coverage, Kind, Material};
pub use review::{Decision, Review};

use serde::{Deserialize, Serialize};
use wire::Wire;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub baseline: Baseline,
    pub candidate: Snapshot,
    pub review: Option<Review>,
    pub reservations: Vec<String>,
    pub major: Option<String>,
}

impl Request {
    pub const SCHEMA: &str = "plumb.release.semantics/v1";
    pub const LIMIT: usize = 4 * 1024 * 1024;

    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > Self::LIMIT {
            return Err("release semantics input exceeds its bounds".into());
        }
        let held: Self = serde_json::from_slice(bytes)
            .map_err(|error| format!("invalid release semantics input: {error}"))?;
        held.validate()?;
        Ok(held)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != Self::SCHEMA {
            return Err("unsupported release semantics schema".into());
        }
        self.baseline.validate()?;
        self.candidate.validate()?;
        if self.candidate.product != self.baseline.stable.snapshot.product {
            return Err("candidate and baseline name different products".into());
        }
        version::Markers(&self.reservations).validate()?;
        if let Some(reference) = &self.major {
            Wire::reference(reference)?;
        }
        if let Some(review) = &self.review {
            review.bind(
                &self.baseline.stable.snapshot,
                &self.candidate,
                &self.baseline.anchor.snapshot,
            )?;
        }
        Ok(())
    }

    pub fn assess(&self) -> Result<Assessment, String> {
        self.validate()?;
        Assessment::new(self)
    }
}
