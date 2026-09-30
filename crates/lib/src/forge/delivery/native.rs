use super::{Narrative, Request, Snapshot, declarations, refuse, squash};
use crate::landing::{Refusal, Request as Landing};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[path = "evidence.rs"]
mod evidence;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use evidence::Evidence;

pub const SCHEMA: &str = "plumb.native-delivery/v1";

pub struct Context<'a> {
    pub root: &'a Path,
    pub source: &'a str,
    pub target: &'a str,
    pub tree: &'a str,
}

pub trait Gate {
    fn verify(&self, context: &Context<'_>) -> Result<Evidence, Refusal>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: String,
    pub root: PathBuf,
    pub repository: String,
    pub issue: Snapshot,
    pub observed: u64,
    pub base: String,
    pub target: String,
    pub branch: String,
    pub projection: String,
    pub source: String,
    pub candidate: String,
    pub pull: Narrative,
    pub evidence: Evidence,
}

pub fn prepare(request: Request<'_>, gate: &impl Gate) -> Result<Plan, Refusal> {
    let issue = declarations(&request)?;
    let inspected = Landing {
        root: request.root,
        base: request.base,
        title: &request.pull.title,
        body: &request.pull.body,
    }
    .inspect()?;
    let draft = inspected.draft();
    let root = &draft.landing.repo.root;
    evidence::admit(root, &draft.source)?;
    evidence::admit(root, &draft.target)?;
    let source = draft
        .landing
        .repo
        .revision(&format!("{}^{{tree}}", draft.source))?;
    if source != draft.tree {
        return Err(refuse(
            "native",
            "projected tree differs from source; update from the exact base before native verification",
        ));
    }
    let context = Context {
        root,
        source: &draft.source,
        target: &draft.target,
        tree: &draft.tree,
    };
    let evidence = gate.verify(&context)?;
    evidence.matches(&context)?;
    draft.landing.repo.clean()?;
    if draft.landing.repo.revision("HEAD")? != draft.source
        || draft.landing.repo.revision(&draft.landing.upstream())? != draft.target
        || draft.landing.repo.branch()? != draft.landing.branch
    {
        return Err(refuse("stale", "repository moved while native gate ran"));
    }
    let token = evidence.encode()?;
    let message = format!(
        "{}\n\n{}\n\nLand-Source: {}@{}\n{} {token}\n",
        draft.story.title.trim(),
        draft.story.body.trim(),
        draft.landing.branch,
        draft.source,
        evidence::TRAILER
    );
    let candidate = draft.record(&message)?;
    if evidence::read(root, &candidate)? != evidence {
        return Err(refuse(
            "native",
            "candidate did not preserve the verified native evidence",
        ));
    }
    Ok(Plan {
        schema: SCHEMA.to_string(),
        root: draft.landing.repo.root,
        repository: request.repository.to_string(),
        issue,
        observed: request.observed,
        base: draft.landing.base,
        target: draft.target,
        branch: draft.landing.branch,
        projection: draft.projection,
        source: draft.source,
        candidate,
        pull: request.pull.clone(),
        evidence,
    })
}

pub fn revalidate(request: Request<'_>, plan: &Plan, gate: &impl Gate) -> Result<Plan, Refusal> {
    if plan.schema != SCHEMA {
        return Err(refuse("native", "unknown native delivery schema"));
    }
    let mut current = prepare(request, gate)?;
    current.observed = plan.observed;
    if &current != plan {
        return Err(refuse(
            "stale",
            "native delivery topology, narrative, candidate or evidence changed",
        ));
    }
    Ok(current)
}

pub fn landed(root: &Path, candidate: &str, head: &str) -> Result<super::Landed, Refusal> {
    let landed = squash::geometry(root, candidate, head)?;
    let expected = evidence::read(root, candidate)?;
    let carried = evidence::read(root, &landed.head)?;
    if carried != expected || expected.tree != landed.tree {
        return Err(refuse(
            "native",
            "squash lost or contradicted the native candidate evidence",
        ));
    }
    Ok(landed)
}
