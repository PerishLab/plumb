use super::{Native, Point, Reference, Transition, wire::Wire};
use crate::seat::depot::v3::Pointer;
use crate::semantics::{Release, Request};
use crate::windtun::qualification::Fault;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Adoption {
    pub product: String,
    pub release: Release,
    pub scenes: Pointer,
    pub world: Reference,
    pub review: Reference,
    pub native: Native,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    pub product: String,
    pub marker: String,
    pub root: Reference,
    pub previous: Reference,
    pub anchor: Reference,
    pub request: Request,
    pub scenes: Pointer,
    pub world: Reference,
    pub review: Option<Reference>,
    pub direct: Option<Reference>,
    pub native: Native,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Candidate,
    Promotion,
    Y,
    Z,
    Unknown,
    Anchor,
    Review,
    Major,
    Revert,
    Stable,
    Direct,
    Retry,
    Existing,
    Pointer,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Trial {
    pub kind: Kind,
    pub point: Point,
    pub native: Native,
    pub observation: Reference,
    pub publication: Option<Reference>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Canary {
    pub product: String,
    pub marker: String,
    pub platform: String,
    pub medium: String,
    pub trials: Vec<Trial>,
    pub native: Native,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Publication {
    pub product: String,
    pub marker: String,
    pub intent: Reference,
    pub world: Reference,
    pub native: Native,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "schema")]
pub enum Record {
    #[serde(rename = "wharf.release.semantics.adoption/v1")]
    Adoption(Box<Adoption>),
    #[serde(rename = "wharf.release.semantics.intent/v1")]
    Intent(Box<Intent>),
    #[serde(rename = "wharf.release.semantics.transition/v1")]
    Transition(Box<Transition>),
    #[serde(rename = "wharf.release.semantics.canary/v1")]
    Canary(Canary),
    #[serde(rename = "wharf.release.semantics.publication/v1")]
    Publication(Publication),
}

impl Record {
    pub fn parse(raw: &[u8]) -> Result<Self, Fault> {
        let held: Self = Wire::parse(raw)?;
        Wire::require(
            Wire::canonical(&held)? == raw,
            "semantic record is not canonical",
        )?;
        held.validate()?;
        Ok(held)
    }

    pub fn encode(&self) -> Result<Vec<u8>, Fault> {
        self.validate()?;
        Wire::canonical(self)
    }

    pub fn validate(&self) -> Result<(), Fault> {
        let product = self.product();
        Wire::name(product)?;
        super::super::version::Markers::parse(self.marker()).map_err(Fault::refused)?;
        self.native().validate()?;
        match self {
            Self::Adoption(held) => {
                super::super::version::Markers::stable(&held.release.marker)
                    .map_err(Fault::refused)?;
                held.release.snapshot.validate().map_err(Fault::refused)?;
                Wire::require(
                    held.release.snapshot.product == product,
                    "adoption snapshot names another product",
                )?;
                held.scenes.encode().map_err(Fault::refused)?;
                held.world.validate(product)?;
                Self::review(&held.review)?;
            }
            Self::Intent(held) => {
                held.request.validate().map_err(Fault::refused)?;
                Wire::require(
                    held.request.candidate.product == product,
                    "intent snapshot names another product",
                )?;
                held.scenes.encode().map_err(Fault::refused)?;
                for reference in [&held.root, &held.previous, &held.anchor, &held.world] {
                    reference.validate(product)?;
                }
                for reference in [&held.review, &held.direct].into_iter().flatten() {
                    Self::review(reference)?;
                }
            }
            Self::Transition(held) => {
                Record::Intent(Box::new(held.intent.clone())).validate()?;
                held.history.validate(product)?;
                if let Some(reference) = &held.source {
                    reference.validate(product)?;
                }
            }
            Self::Publication(held) => {
                held.intent.validate(product)?;
                held.world.validate(product)?;
            }
            Self::Canary(held) => {
                Wire::name(&held.platform)?;
                Wire::name(&held.medium)?;
                Wire::require(
                    !held.trials.is_empty() && held.trials.len() <= 64,
                    "canary observations exceed bounds",
                )?;
                for trial in &held.trials {
                    Self::trial(trial, product)?;
                }
            }
        }
        Ok(())
    }

    fn review(reference: &Reference) -> Result<(), Fault> {
        super::super::wire::Wire::reference(&reference.url).map_err(Fault::refused)?;
        Wire::hex(&reference.sha256, 64)?;
        Wire::require(
            reference.size > 0 && reference.size <= 64 * 1024,
            "semantic review reference exceeds bounds",
        )
    }

    fn trial(trial: &Trial, product: &str) -> Result<(), Fault> {
        Wire::require(
            trial.point.repository == format!("PerishLab/{product}"),
            "canary observation names another product",
        )?;
        super::super::version::Markers::parse(&trial.point.marker).map_err(Fault::refused)?;
        Wire::hex(&trial.point.commit, 40)?;
        Wire::hex(&trial.point.tree, 40)?;
        trial.native.validate()?;
        trial.observation.validate(product)?;
        if let Some(reference) = &trial.publication {
            reference.validate(product)?;
        }
        Ok(())
    }

    pub fn product(&self) -> &str {
        match self {
            Self::Adoption(held) => &held.product,
            Self::Intent(held) => &held.product,
            Self::Transition(held) => &held.intent.product,
            Self::Canary(held) => &held.product,
            Self::Publication(held) => &held.product,
        }
    }

    pub fn marker(&self) -> &str {
        match self {
            Self::Adoption(held) => &held.release.marker,
            Self::Intent(held) => &held.marker,
            Self::Transition(held) => &held.intent.marker,
            Self::Canary(held) => &held.marker,
            Self::Publication(held) => &held.marker,
        }
    }

    pub fn native(&self) -> &Native {
        match self {
            Self::Adoption(held) => &held.native,
            Self::Intent(held) => &held.native,
            Self::Transition(held) => &held.intent.native,
            Self::Canary(held) => &held.native,
            Self::Publication(held) => &held.native,
        }
    }

    pub fn release(&self) -> Result<Release, Fault> {
        super::super::version::Markers::stable(self.marker()).map_err(Fault::refused)?;
        match self {
            Self::Adoption(held) => Ok(held.release.clone()),
            Self::Intent(held) => Ok(Release {
                marker: held.marker.clone(),
                snapshot: held.request.candidate.clone(),
            }),
            Self::Transition(held) => Ok(Release {
                marker: held.intent.marker.clone(),
                snapshot: held.intent.request.candidate.clone(),
            }),
            Self::Canary(_) | Self::Publication(_) => Err(Fault::refused(
                "evidence reference is not a formal product baseline",
            )),
        }
    }
}
