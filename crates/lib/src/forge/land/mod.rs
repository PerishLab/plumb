use super::github::{self, Client};
use serde::Serialize;
use std::path::{Path, PathBuf};

mod flow;
pub mod rejoin;
mod repo;

use flow::{Candidate, Landing};

pub use crate::landing::{Guard, Preparation, Refusal};

pub const SCHEMA: &str = "plumb.land/v1";

pub struct Request<'a> {
    pub root: &'a Path,
    pub base: &'a str,
    pub title: &'a str,
    pub body: &'a str,
    pub watch: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub root: PathBuf,
    pub base: String,
    pub branch: String,
    pub projection: String,
    pub source: String,
    pub candidate: String,
    #[serde(rename = "pull_node")]
    pub node: String,
    pub pull: u64,
    pub url: String,
    pub merged: bool,
    pub retired: crate::delivery::Retired,
    pub synced: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Plan {
    pub schema: &'static str,
    pub root: PathBuf,
    pub base: String,
    pub branch: String,
    pub projection: String,
    pub steps: Vec<String>,
}

pub(crate) fn refuse(kind: &'static str, message: impl Into<String>) -> Refusal {
    crate::landing::refuse(kind, message)
}

impl Request<'_> {
    pub fn prepare(self) -> Result<Preparation, Refusal> {
        prepare(self.root, self.base, self.title, self.body)
    }
}

fn prepare(root: &Path, base: &str, title: &str, body: &str) -> Result<Preparation, Refusal> {
    let inspected = crate::landing::Request {
        root,
        base,
        title,
        body,
    }
    .inspect()?;
    let verified = crate::guard::running::verified(inspected.root(), inspected.source())
        .map_err(|error| refuse("guard", error))?;
    inspected.prepare(verified)
}

pub fn plan(request: Request<'_>) -> Result<Plan, Refusal> {
    let landing = Landing::open(request.root, request.base)?;
    landing.landable(false)?;
    let remote = github::remote(&landing.repo.root).map_err(|error| refuse("remote", error))?;
    let seat = format!("{}/{}", remote.owner, remote.repo);
    let base = &landing.base;
    let branch = &landing.branch;
    let projection = landing.projection();
    let steps = vec![
        "git fetch --prune origin".to_string(),
        format!("verify {branch} is clean, outside a release line, and differs from origin/{base}"),
        format!("derive one commit at {projection} from origin/{base} plus {branch}"),
        format!("git push -u origin {branch}"),
        format!("git push --force-with-lease origin <candidate>:refs/heads/{projection}"),
        format!("gh pr list -R {seat} --state open --base {base} --head {projection}"),
        format!("gh pr create -R {seat} --base {base} --head {projection} if missing"),
        "verify <candidate> carries the staged-tree guard proof".to_string(),
        format!(
            "gh api -X POST repos/{seat}/statuses/<candidate> (context=guard / guard (pull_request), state=success)"
        ),
        format!(
            "gh pr merge <n> -R {seat} --squash --match-head-commit <candidate> --subject <its subject> --body <its body>"
        ),
        format!(
            "fetch origin and verify origin/{base} is one squash carrying <candidate>'s parent, tree and Guard proof"
        ),
        format!(
            "delete {projection} and {branch} on origin by lease on the heads pushed, and locally while they hold them"
        ),
        format!("exactly fast-forward the clean {base} worktree to origin/{base}"),
    ];
    Ok(Plan {
        schema: SCHEMA,
        root: landing.repo.root.clone(),
        base: base.clone(),
        branch: branch.clone(),
        projection,
        steps,
    })
}

pub fn run(request: Request<'_>) -> Result<Report, Refusal> {
    execute(request, None)
}

impl Request<'_> {
    pub fn exact(self, expected: &Preparation) -> Result<Report, Refusal> {
        execute(self, Some(expected))
    }
}

fn execute(request: Request<'_>, expected: Option<&Preparation>) -> Result<Report, Refusal> {
    let landing = Landing::open(request.root, request.base)?;
    landing.landable(true)?;
    let remote = github::remote(&landing.repo.root).map_err(|error| refuse("remote", error))?;
    let client = Client::new(&remote);
    let projection = landing.projection();
    let standing = client
        .opened(&landing.base, &projection)
        .map_err(|error| refuse("forge", error))?;
    let title = standing
        .as_ref()
        .filter(|_| request.title.is_empty())
        .map_or(request.title, |pull| &pull.title);
    let body = standing
        .as_ref()
        .filter(|_| request.body.is_empty())
        .map_or(request.body, |pull| &pull.body);
    let story = landing.describe(title, body)?;
    let prepared = prepare(&landing.repo.root, &landing.base, &story.title, &story.body)?;
    let verified = crate::guard::running::verified(&landing.repo.root, &prepared.source)
        .map_err(|error| refuse("guard", error))?;
    let prepared = crate::landing::Request {
        root: &landing.repo.root,
        base: &landing.base,
        title: &story.title,
        body: &story.body,
    }
    .revalidate(expected.unwrap_or(&prepared), verified)?
    .take();
    let candidate = Candidate {
        projection: prepared.projection.clone(),
        head: prepared.candidate.clone(),
        source: prepared.source.clone(),
        base: prepared.target.clone(),
    };
    if expected.is_some()
        && standing
            .as_ref()
            .is_some_and(|pull| pull.title != story.title || pull.body != story.body)
    {
        return Err(refuse(
            "stale",
            "standing pull narrative no longer matches the delivery plan",
        ));
    }

    landing.push(&landing.branch, &landing.branch, true)?;
    if standing
        .as_ref()
        .is_none_or(|pull| pull.head != candidate.head)
    {
        landing.push(&candidate.projection, &candidate.head, false)?;
    }

    let pull = match standing {
        Some(held) => held,
        None => client
            .raise(
                &landing.base,
                &candidate.projection,
                &story.title,
                &story.body,
            )
            .map_err(|error| refuse("forge", error))?,
    };

    let mut report = shape(&landing, &candidate, &pull);
    if !request.watch {
        return Ok(report);
    }
    landing.settled(&candidate)?;
    let squash = crate::delivery::Squash::read(&landing.repo.root, &candidate.head)?;
    client
        .mark(
            &candidate.head,
            "guard / guard (pull_request)",
            "Plumb verified the staged-tree Guard proof",
        )
        .map_err(|error| refuse("forge", error))?;
    client
        .settle(pull.number, &squash)
        .map_err(|error| refuse("forge", error))?;
    landing.repo.fetch()?;
    let head = landing.repo.revision(&landing.upstream())?;
    let delivered = crate::delivery::Delivered {
        root: &landing.repo.root,
        remote: "origin",
        candidate: &candidate.head,
        head: &head,
    };
    let pushed = [
        crate::delivery::Pushed {
            branch: candidate.projection.clone(),
            head: candidate.head.clone(),
        },
        crate::delivery::Pushed {
            branch: landing.branch.clone(),
            head: candidate.source.clone(),
        },
    ];
    let (_, retired) = crate::delivery::retired(&delivered, &pushed)?;
    report.merged = true;
    report.retired = retired;
    report.synced = landing.sync()?;
    Ok(report)
}

fn shape(landing: &Landing, candidate: &Candidate, pull: &github::Pull) -> Report {
    Report {
        schema: SCHEMA,
        root: landing.repo.root.clone(),
        base: landing.base.clone(),
        branch: landing.branch.clone(),
        projection: candidate.projection.clone(),
        source: candidate.source.clone(),
        candidate: candidate.head.clone(),
        node: pull.node.clone(),
        pull: pull.number,
        url: pull.url.clone(),
        merged: false,
        retired: crate::delivery::Retired::default(),
        synced: None,
    }
}
