use super::artifact::{Artifact, Listing, Slot};
use super::request::{Client, Scan};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

#[derive(Deserialize)]
struct Repository {
    #[serde(rename = "default_branch")]
    branch: String,
}

#[derive(Deserialize)]
pub(super) struct Run {
    pub path: String,
    pub event: String,
    pub status: String,
    pub conclusion: Option<String>,
    #[serde(rename = "head_sha")]
    pub head: String,
    #[serde(rename = "run_attempt")]
    pub attempt: u64,
    #[serde(default, rename = "pull_requests")]
    pub requests: Vec<Request>,
}

#[derive(Deserialize)]
pub(super) struct Request {
    pub number: u64,
}

#[derive(Deserialize)]
struct Commit {
    tree: Object,
}

#[derive(Deserialize)]
struct Object {
    sha: String,
}

pub(crate) struct Lookup<'a> {
    pub root: &'a Path,
    pub target: &'a Path,
    pub family: &'a str,
    pub exact: &'a str,
    pub budget: Duration,
    pub pages: u64,
}

pub(crate) fn fetch(lookup: &Lookup<'_>) -> Result<bool, String> {
    let Some(client) = Client::new(lookup.root, lookup.budget)? else {
        return Ok(false);
    };
    let repository = repository(lookup.root)?;
    let metadata: Repository = client.json(&format!("repos/{repository}"))?;
    let trees = trees(lookup.root, &metadata.branch)?;
    let mut scan = Scan::default();
    let mut found = Vec::new();
    let mut start = 0;
    let mut hunt = Hunt {
        client: &client,
        repository: &repository,
        lookup,
        trees: &trees,
        pull: pull(),
        scan: &mut scan,
    };
    for page in 1..=lookup.pages {
        let listing = match artifacts(hunt.client, hunt.repository, page) {
            Ok(listing) => listing,
            Err(error) if exhausted(&error) && !found.is_empty() => break,
            Err(error) => return Err(error),
        };
        let count = listing.artifacts.len();
        hunt.scan.pages = page;
        for artifact in listing.artifacts {
            let slot = artifact.slot(hunt.lookup.family, hunt.lookup.exact);
            hunt.scan.note(
                &artifact.name,
                artifact.expired,
                slot.as_ref(),
                hunt.lookup.family,
            );
            if let Some(slot) = slot {
                found.push((artifact, slot));
            }
        }
        if offer(&mut hunt, &found, start, true)? {
            return Ok(true);
        }
        start = found.len();
        if count < 20 {
            break;
        }
    }
    if offer(&mut hunt, &found, 0, false)? {
        return Ok(true);
    }
    hunt.scan.finish(false);
    Ok(false)
}

struct Hunt<'a> {
    client: &'a Client,
    repository: &'a str,
    lookup: &'a Lookup<'a>,
    trees: &'a BTreeSet<String>,
    pull: Option<u64>,
    scan: &'a mut Scan,
}

fn artifacts(client: &Client, repository: &str, page: u64) -> Result<Listing, String> {
    client.json(&format!(
        "repos/{repository}/actions/artifacts?per_page=20&page={page}"
    ))
}

fn exhausted(error: &str) -> bool {
    error.contains("exceeded its") && error.contains("budget")
}

fn offer(
    hunt: &mut Hunt<'_>,
    found: &[(Artifact, Slot)],
    start: usize,
    exact: bool,
) -> Result<bool, String> {
    for (artifact, slot) in found.iter().skip(start) {
        if slot.exact == exact && take(hunt, artifact, slot)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn take(hunt: &mut Hunt<'_>, artifact: &Artifact, slot: &Slot) -> Result<bool, String> {
    let attempt = slot.attempt;
    let Some(reference) = &artifact.run else {
        hunt.scan.refused += 1;
        return Ok(false);
    };
    let run: Run = hunt.client.json(&format!(
        "repos/{}/actions/runs/{}/attempts/{attempt}",
        hunt.repository, reference.id
    ))?;
    if !run.admitted(attempt) {
        hunt.scan.refused += 1;
        return Ok(false);
    }
    let commit: Commit = hunt.client.json(&format!(
        "repos/{}/git/commits/{}",
        hunt.repository, run.head
    ))?;
    let mut requests = run.requests();
    if !belongs(hunt.trees, &commit.tree.sha, hunt.pull, &requests)
        && hunt.pull.is_some()
        && requests.is_empty()
    {
        requests = linked(hunt.client, hunt.repository, &run.head)?;
    }
    if !belongs(hunt.trees, &commit.tree.sha, hunt.pull, &requests) {
        hunt.scan.outside += 1;
        return Ok(false);
    }
    artifact.download(hunt.client, hunt.repository, hunt.lookup.target)?;
    let producer = if slot.exact { "this" } else { "another" };
    if hunt.trees.contains(&commit.tree.sha) {
        eprintln!(
            "guard material: fetched {producer} producer's material for accepted default-branch tree {} from Guard run {} attempt {attempt}",
            commit.tree.sha, reference.id
        );
    } else {
        eprintln!(
            "guard material: fetched {producer} producer's material for this pull request from Guard run {} attempt {attempt}",
            reference.id
        );
    }
    hunt.scan.finish(true);
    Ok(true)
}

impl Run {
    pub(super) fn admitted(&self, attempt: u64) -> bool {
        if self.status != "completed" || self.conclusion.as_deref() != Some("success") {
            return false;
        }
        if self.path != ".github/workflows/guard.yml" || self.attempt != attempt {
            return false;
        }
        matches!(self.event.as_str(), "pull_request" | "merge_group") && object(&self.head)
    }

    fn requests(&self) -> Vec<u64> {
        self.requests
            .iter()
            .map(|request| request.number)
            .filter(|number| *number != 0)
            .collect()
    }
}

fn pull() -> Option<u64> {
    std::env::var("GITHUB_REF")
        .ok()
        .as_deref()
        .and_then(numbered)
}

pub(super) fn numbered(value: &str) -> Option<u64> {
    let rest = value.strip_prefix("refs/pull/")?;
    let (number, suffix) = rest.split_once('/')?;
    if suffix != "merge" && suffix != "head" {
        return None;
    }
    number.parse().ok().filter(|number| *number != 0)
}

pub(super) fn belongs(
    trees: &BTreeSet<String>,
    tree: &str,
    pull: Option<u64>,
    requests: &[u64],
) -> bool {
    trees.contains(tree) || pull.is_some_and(|pull| requests.contains(&pull))
}

fn linked(client: &Client, repository: &str, head: &str) -> Result<Vec<u64>, String> {
    let requests: Vec<Request> =
        client.json(&format!("repos/{repository}/commits/{head}/pulls"))?;
    Ok(requests
        .iter()
        .map(|request| request.number)
        .filter(|number| *number != 0)
        .collect())
}

fn repository(root: &Path) -> Result<String, String> {
    let output = plumb::config::detached("git")
        .arg("-C")
        .arg(root)
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(|_| "cannot read cache repository")?;
    let text = std::str::from_utf8(&output.stdout)
        .map_err(|_| "cache origin is not text")?
        .trim();
    let path = text
        .strip_prefix("https://github.com/")
        .or_else(|| text.strip_prefix("git@github.com:"))
        .ok_or("cache heat requires canonical github.com origin")?
        .trim_end_matches(".git");
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.len() != 2
        || !parts.iter().all(|part| {
            !part.is_empty()
                && !matches!(*part, "." | "..")
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
        })
    {
        return Err("cache repository identity is invalid".into());
    }
    Ok(path.into())
}

fn trees(root: &Path, branch: &str) -> Result<BTreeSet<String>, String> {
    let reference = format!("refs/remotes/origin/{branch}");
    let output = plumb::config::detached("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--format=%T", "--max-count=50", &reference])
        .output()
        .map_err(|_| "cannot read accepted default-branch trees")?;
    if !output.status.success() {
        return Err("accepted default-branch history is unavailable locally".into());
    }
    let text =
        std::str::from_utf8(&output.stdout).map_err(|_| "default-branch tree list is invalid")?;
    let trees = text.lines().map(str::to_string).collect::<BTreeSet<_>>();
    if trees.is_empty() || !trees.iter().all(|tree| object(tree)) {
        return Err("default-branch trees are invalid".into());
    }
    Ok(trees)
}

fn object(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
