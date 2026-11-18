use super::artifact::Listing;
use super::request::{Client, Scan};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Deserialize)]
struct Repository {
    #[serde(rename = "default_branch")]
    branch: String,
}

#[derive(Deserialize)]
struct Run {
    path: String,
    event: String,
    status: String,
    conclusion: Option<String>,
    #[serde(rename = "head_sha")]
    head: String,
    #[serde(rename = "run_attempt")]
    attempt: u64,
}

#[derive(Deserialize)]
struct Commit {
    tree: Object,
}

#[derive(Deserialize)]
struct Object {
    sha: String,
}

pub(crate) fn fetch(root: &Path, target: &Path, family: &str, exact: &str) -> Result<bool, String> {
    let Some(client) = Client::new(root)? else {
        return Ok(false);
    };
    let repository = repository(root)?;
    let metadata: Repository = client.json(&format!("repos/{repository}"))?;
    let trees = trees(root, &metadata.branch)?;
    let mut scan = Scan::default();
    let mut found = Vec::new();
    for page in 1..=5 {
        let listing: Listing = client.json(&format!(
            "repos/{repository}/actions/artifacts?per_page=20&page={page}"
        ))?;
        let count = listing.artifacts.len();
        scan.pages = page;
        for artifact in listing.artifacts {
            let slot = artifact.slot(family, exact);
            scan.note(&artifact.name, artifact.expired, slot.as_ref(), family);
            if let Some(slot) = slot {
                found.push((artifact, slot));
            }
        }
        if count < 20 {
            break;
        }
    }
    found.sort_by_key(|(_, slot)| !slot.exact);
    for (artifact, slot) in found {
        let attempt = slot.attempt;
        let Some(reference) = &artifact.run else {
            scan.refused += 1;
            continue;
        };
        let run: Run = client.json(&format!(
            "repos/{repository}/actions/runs/{}/attempts/{attempt}",
            reference.id
        ))?;
        if !run.admitted(attempt) {
            scan.refused += 1;
            continue;
        }
        let commit: Commit =
            client.json(&format!("repos/{repository}/git/commits/{}", run.head))?;
        if !trees.contains(&commit.tree.sha) {
            scan.outside += 1;
            continue;
        }
        artifact.download(&client, &repository, target)?;
        let producer = if slot.exact { "this" } else { "another" };
        eprintln!(
            "guard material: fetched {producer} producer's material for accepted default-branch tree {} from Guard run {} attempt {attempt}",
            commit.tree.sha, reference.id
        );
        scan.finish(true);
        return Ok(true);
    }
    scan.finish(false);
    Ok(false)
}

impl Run {
    fn admitted(&self, attempt: u64) -> bool {
        if self.status != "completed" || self.conclusion.as_deref() != Some("success") {
            return false;
        }
        if self.path != ".github/workflows/guard.yml" || self.attempt != attempt {
            return false;
        }
        matches!(self.event.as_str(), "pull_request" | "merge_group") && object(&self.head)
    }
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

#[cfg(test)]
mod tests {
    use super::Run;

    #[test]
    fn admission() {
        let mut run = Run {
            path: ".github/workflows/guard.yml".into(),
            event: "pull_request".into(),
            status: "completed".into(),
            conclusion: Some("success".into()),
            head: "a".repeat(40),
            attempt: 2,
        };
        assert!(run.admitted(2));
        assert!(!run.admitted(1));
        run.conclusion = Some("failure".into());
        assert!(!run.admitted(2));
        run.conclusion = Some("success".into());
        run.path = ".github/workflows/foreign.yml".into();
        assert!(!run.admitted(2));
        run.path = ".github/workflows/guard.yml".into();
        run.event = "workflow_dispatch".into();
        assert!(!run.admitted(2));
    }
}
