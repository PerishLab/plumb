use crate::guard::{Authority, Expected};
use crate::landing::{self, Preparation, Refusal};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod native;
mod retire;
mod squash;

pub use retire::{Delivered, Kept, Pushed, Retired, retired};
pub use squash::{Landed, Squash, landed};

pub const SCHEMA: &str = "plumb.delivery-plan/v2";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub node: String,
    pub repository: String,
    pub number: u64,
    pub url: String,
    pub title: String,
    pub state: String,
    pub kind: String,
    pub updated: String,
    pub parent: Option<Reference>,
    pub sub_issues: Vec<Reference>,
    pub blocked_by: Vec<Reference>,
    pub blocking: Vec<Reference>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub repository: String,
    pub number: u64,
    pub node: String,
    pub url: String,
    pub title: String,
    pub state: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Narrative {
    pub title: String,
    pub body: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: String,
    pub root: PathBuf,
    pub repository: String,
    pub issue: Snapshot,
    #[serde(rename = "observed_at")]
    pub observed: u64,
    pub base: String,
    #[serde(rename = "base_commit")]
    pub target: String,
    pub branch: String,
    pub projection: String,
    pub source: String,
    pub candidate: String,
    pub pull: Narrative,
    pub guard: landing::Guard,
}

pub struct Request<'a> {
    pub root: &'a Path,
    pub repository: &'a str,
    pub issue: &'a Snapshot,
    pub observed: u64,
    pub base: &'a str,
    pub pull: &'a Narrative,
}

pub struct Ready(landing::Ready);

#[derive(Eq, PartialEq)]
struct Contract<'a> {
    repository: &'a str,
    issue: &'a Snapshot,
    base: &'a str,
    pull: &'a Narrative,
}

impl Plan {
    pub fn agrees(&self, request: &Request<'_>) -> Result<bool, Refusal> {
        let issue = declarations(request)?;
        let current = Contract {
            repository: request.repository,
            issue: &issue,
            base: request.base,
            pull: request.pull,
        };
        let expected = Contract {
            repository: &self.repository,
            issue: &self.issue,
            base: &self.base,
            pull: &self.pull,
        };
        Ok(current == expected)
    }
}

pub fn prepare(request: Request<'_>, authority: &Authority) -> Result<Plan, Refusal> {
    let issue = declarations(&request)?;
    let inspected = landing::Request {
        root: request.root,
        base: request.base,
        title: &request.pull.title,
        body: &request.pull.body,
    }
    .inspect()?;
    let proof = crate::guard::commit(inspected.root(), inspected.source())
        .map_err(|error| refuse("guard", error))?;
    let expected = Expected::new(&proof.schema, &proof.tree, &proof.digest);
    let verified = authority
        .verify(inspected.root(), inspected.source(), &expected)
        .map_err(|error| refuse("guard", error))?;
    let prepared = inspected.prepare(verified)?;
    Ok(shape(request, issue, prepared))
}

pub fn revalidate(
    request: Request<'_>,
    plan: &Plan,
    authority: &Authority,
) -> Result<Ready, Refusal> {
    if plan.schema != SCHEMA {
        return Err(refuse(
            "plan",
            format!("delivery plan schema {} is not {SCHEMA}", plan.schema),
        ));
    }
    if !plan.agrees(&request)? {
        return Err(refuse(
            "stale",
            "delivery Issue snapshot, repository, base, or pull narrative changed",
        ));
    }
    let prepared = preparation(plan);
    let inspected = landing::Request {
        root: request.root,
        base: request.base,
        title: &request.pull.title,
        body: &request.pull.body,
    }
    .inspect()?;
    let guard = Expected::new(&plan.guard.schema, &plan.guard.tree, &plan.guard.digest);
    let verified = authority
        .verify(inspected.root(), inspected.source(), &guard)
        .map_err(|error| refuse("guard", error))?;
    landing::Request {
        root: request.root,
        base: request.base,
        title: &request.pull.title,
        body: &request.pull.body,
    }
    .revalidate(&prepared, verified)
    .map(Ready)
}

pub fn read(path: &Path) -> Result<Plan, Refusal> {
    let bytes = std::fs::read(path).map_err(|error| {
        refuse(
            "plan",
            format!("cannot read delivery plan {}: {error}", path.display()),
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| refuse("plan", format!("cannot parse delivery plan: {error}")))
}

impl Ready {
    pub fn preparation(&self) -> &Preparation {
        self.0.preparation()
    }
}

fn declarations(request: &Request<'_>) -> Result<Snapshot, Refusal> {
    let mut issue = request.issue.clone();
    issue.sub_issues.sort();
    issue.blocked_by.sort();
    issue.blocking.sort();
    if request.observed == 0 {
        return Err(refuse("issue", "Issue snapshot has no observation time"));
    }
    if request.repository.trim().is_empty() || issue.repository != request.repository {
        return Err(refuse(
            "issue",
            "Issue snapshot coordinate does not match the delivery repository",
        ));
    }
    validate(&issue)?;
    if request.pull.title.trim().is_empty() || request.pull.body.trim().is_empty() {
        return Err(refuse("pull", "pull narrative requires a title and body"));
    }
    Ok(issue)
}

fn validate(issue: &Snapshot) -> Result<(), Refusal> {
    for (name, value) in [
        ("node", issue.node.as_str()),
        ("repository", issue.repository.as_str()),
        ("url", issue.url.as_str()),
        ("title", issue.title.as_str()),
        ("type", issue.kind.as_str()),
        ("update identity", issue.updated.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(refuse("issue", format!("Issue snapshot has no {name}")));
        }
    }
    if issue.number == 0 || issue.state != "OPEN" {
        return Err(refuse("issue", "Issue snapshot is not one open coordinate"));
    }
    let mut relations = issue.sub_issues.clone();
    relations.extend(issue.blocked_by.clone());
    relations.extend(issue.blocking.clone());
    if let Some(parent) = &issue.parent {
        relations.push(parent.clone());
    }
    for held in relations {
        relation(&held)?;
    }
    Ok(())
}

fn relation(relation: &Reference) -> Result<(), Refusal> {
    if relation.number == 0 {
        return Err(refuse("issue", "Issue relationship has no coordinate"));
    }
    for value in [
        relation.repository.as_str(),
        relation.node.as_str(),
        relation.url.as_str(),
        relation.title.as_str(),
    ] {
        if value.trim().is_empty() {
            return Err(refuse("issue", "Issue relationship has an empty identity"));
        }
    }
    if !matches!(relation.state.as_str(), "OPEN" | "CLOSED") {
        return Err(refuse("issue", "Issue relationship has an invalid state"));
    }
    Ok(())
}

fn shape(request: Request<'_>, issue: Snapshot, prepared: Preparation) -> Plan {
    Plan {
        schema: SCHEMA.to_string(),
        root: prepared.root,
        repository: request.repository.to_string(),
        issue,
        observed: request.observed,
        base: prepared.base,
        target: prepared.target,
        branch: prepared.branch,
        projection: prepared.projection,
        source: prepared.source,
        candidate: prepared.candidate,
        pull: request.pull.clone(),
        guard: prepared.guard,
    }
}

fn preparation(plan: &Plan) -> Preparation {
    Preparation {
        root: plan.root.clone(),
        base: plan.base.clone(),
        target: plan.target.clone(),
        branch: plan.branch.clone(),
        projection: plan.projection.clone(),
        source: plan.source.clone(),
        candidate: plan.candidate.clone(),
        title: plan.pull.title.clone(),
        body: plan.pull.body.clone(),
        guard: plan.guard.clone(),
    }
}

fn refuse(kind: &'static str, message: impl Into<String>) -> Refusal {
    landing::refuse(kind, message)
}
