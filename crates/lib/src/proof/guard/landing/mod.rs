use crate::guard::{Descriptor, Verified};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

mod flow;
mod repo;
#[cfg(test)]
mod tests;

use flow::{Draft, Landing};

pub const SCHEMA: &str = "plumb.landing/v1";

pub struct Request<'a> {
    pub root: &'a Path,
    pub base: &'a str,
    pub title: &'a str,
    pub body: &'a str,
}

pub struct Inspection(Draft);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Preparation {
    pub root: PathBuf,
    pub base: String,
    pub target: String,
    pub branch: String,
    pub projection: String,
    pub source: String,
    pub candidate: String,
    pub title: String,
    pub body: String,
    pub guard: Guard,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Guard {
    pub schema: String,
    pub tree: String,
    pub digest: String,
}

#[derive(Debug)]
pub struct Ready(Preparation);

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Refusal {
    pub kind: &'static str,
    pub message: String,
}

impl Display for Refusal {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for Refusal {}

pub(crate) fn refuse(kind: &'static str, message: impl Into<String>) -> Refusal {
    Refusal {
        kind,
        message: message.into(),
    }
}

impl Request<'_> {
    pub fn inspect(self) -> Result<Inspection, Refusal> {
        Landing::open(self.root, self.base)?
            .inspect(self.title, self.body)
            .map(Inspection)
    }

    pub fn revalidate(self, expected: &Preparation, verified: Verified) -> Result<Ready, Refusal> {
        let current = self.inspect()?.prepare(verified)?;
        if &current != expected {
            return Err(refuse(
                "stale",
                "landing preparation no longer matches its repository, topology, narrative, candidate, or Guard authority",
            ));
        }
        Ok(Ready(current))
    }
}

impl Inspection {
    pub fn root(&self) -> &Path {
        &self.0.landing.repo.root
    }

    pub fn source(&self) -> &str {
        &self.0.source
    }

    pub fn prepare(self, verified: Verified) -> Result<Preparation, Refusal> {
        let proof = verified.descriptor();
        exact(&self.0, proof)?;
        let candidate = self.0.candidate(proof)?;
        let carried = crate::guard::commit(&self.0.landing.repo.root, &candidate)
            .map_err(|error| refuse("guard", error))?;
        if &carried != proof {
            return Err(refuse(
                "guard",
                "candidate does not carry the verified source Guard proof",
            ));
        }
        Ok(shape(self.0, candidate, proof))
    }
}

impl Ready {
    pub fn preparation(&self) -> &Preparation {
        &self.0
    }

    pub fn take(self) -> Preparation {
        self.0
    }
}

fn exact(draft: &Draft, proof: &Descriptor) -> Result<(), Refusal> {
    let carried = crate::guard::commit(&draft.landing.repo.root, &draft.source)
        .map_err(|error| refuse("guard", error))?;
    if &carried != proof {
        return Err(refuse(
            "guard",
            "verified Guard evidence does not belong to the inspected source",
        ));
    }
    if proof.tree != draft.tree {
        return Err(refuse(
            "guard",
            format!(
                "the guarded source tree {} does not equal the projected tree {}; update the source from {} and run precommit again",
                proof.tree,
                draft.tree,
                draft.landing.upstream()
            ),
        ));
    }
    Ok(())
}

fn shape(draft: Draft, candidate: String, proof: &Descriptor) -> Preparation {
    Preparation {
        root: draft.landing.repo.root,
        base: draft.landing.base,
        target: draft.target,
        branch: draft.landing.branch,
        projection: draft.projection,
        source: draft.source,
        candidate,
        title: draft.story.title,
        body: draft.story.body,
        guard: Guard {
            schema: proof.schema.clone(),
            tree: proof.tree.clone(),
            digest: proof.digest.clone(),
        },
    }
}
