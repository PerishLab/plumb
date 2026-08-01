use super::{Client, Strategy, git, harness};
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

mod flow;
mod repo;

use flow::{Candidate, Landing};

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
    pub pull: u64,
    pub url: String,
    pub merged: bool,
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

pub fn plan(request: Request<'_>) -> Result<Plan, Refusal> {
    let landing = Landing::open(request.root, request.base)?;
    landing.landable(false)?;
    let remote = git::remote(&landing.repo.root, "").map_err(|error| refuse("remote", error))?;
    let seat = format!(
        "{}://{}/api/v1/repos/{}/{}",
        remote.scheme, remote.host, remote.owner, remote.repo
    );
    let base = &landing.base;
    let branch = &landing.branch;
    let projection = landing.projection();
    let steps = vec![
        "git fetch --prune origin".to_string(),
        format!("verify {branch} is clean, outside a release line, and differs from origin/{base}"),
        format!("derive one commit at {projection} from origin/{base} plus {branch}"),
        format!("git push -u origin {branch}"),
        format!("git push --force-with-lease origin <candidate>:refs/heads/{projection}"),
        format!("GET {seat}/pulls?state=open"),
        format!("POST {seat}/pulls (base={base}, head={projection}) if missing"),
        format!("GET {seat}/commits/<candidate>/status until guard succeeds"),
        format!(
            "POST {seat}/pulls/<n>/merge (Do=fast-forward-only, head_commit_id=<candidate>, delete_branch_after_merge=false)"
        ),
        format!("git pull --ff-only origin {base} in the worktree holding {base}"),
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
    let landing = Landing::open(request.root, request.base)?;
    landing.landable(true)?;
    let remote = git::remote(&landing.repo.root, "").map_err(|error| refuse("remote", error))?;
    let story = landing.describe(request.title, request.body)?;
    let candidate = landing.derive(&story)?;

    landing.push(&landing.branch, &landing.branch, true)?;
    landing.push(&candidate.projection, &candidate.head, false)?;

    let client = Client::new(remote).map_err(|error| refuse("forge", error))?;
    let pull = match client
        .opened(&landing.base, &candidate.projection)
        .map_err(|error| refuse("forge", error))?
    {
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

    let mut report = shape(&landing, &candidate, pull.number, &pull.url);
    if !request.watch {
        return Ok(report);
    }
    guard(&client, &candidate.head, pull.number)?;
    landing.settled(&candidate)?;
    client
        .settle(pull.number, &candidate.head, Strategy::Forward)
        .map_err(|error| refuse("forge", error))?;
    report.merged = true;
    report.synced = landing.sync()?;
    Ok(report)
}

fn shape(landing: &Landing, candidate: &Candidate, pull: u64, url: &str) -> Report {
    Report {
        schema: SCHEMA,
        root: landing.repo.root.clone(),
        base: landing.base.clone(),
        branch: landing.branch.clone(),
        projection: candidate.projection.clone(),
        source: candidate.source.clone(),
        candidate: candidate.head.clone(),
        pull,
        url: url.to_string(),
        merged: false,
        synced: None,
    }
}

fn guard(client: &Client, head: &str, pull: u64) -> Result<(), Refusal> {
    let knobs = harness().map_err(|error| refuse("forge", error))?;
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_millis(knobs.guard_timeout_ms);
    while std::time::Instant::now() < deadline {
        let state = client
            .combined(head)
            .map_err(|error| refuse("forge", error))?;
        if state.count == 0 {
            std::thread::sleep(std::time::Duration::from_millis(knobs.guard_register_ms));
            continue;
        }
        if state.state == "success" {
            return Ok(());
        }
        if state.state != "pending" {
            return Err(refuse(
                "guardfailed",
                format!("guard {} on {head}; pull #{pull} left open", state.state),
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(knobs.guard_pending_ms));
    }
    Err(refuse(
        "guardpending",
        format!(
            "guard still pending on {head} after {}s; pull #{pull} left open",
            knobs.guard_timeout_ms / 1000
        ),
    ))
}
