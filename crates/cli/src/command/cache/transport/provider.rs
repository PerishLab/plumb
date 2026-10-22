use super::inputs::digest;
use super::request::Client;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Deserialize)]
struct Repository {
    #[serde(rename = "default_branch")]
    branch: String,
}

#[derive(Deserialize)]
struct Listing {
    artifacts: Vec<Artifact>,
}

#[derive(Deserialize)]
struct Artifact {
    id: u64,
    name: String,
    #[serde(rename = "size_in_bytes")]
    size: u64,
    digest: Option<String>,
    expired: bool,
    #[serde(rename = "workflow_run")]
    run: Option<Reference>,
}

#[derive(Deserialize)]
struct Reference {
    id: u64,
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

pub(crate) fn fetch(root: &Path, target: &Path, prefix: &str) -> Result<bool, String> {
    let Some(client) = Client::new(root)? else {
        return Ok(false);
    };
    let repository = repository(root)?;
    let metadata: Repository = client.json(&format!("repos/{repository}"))?;
    let trees = trees(root, &metadata.branch)?;
    for page in 1..=5 {
        let listing: Listing = client.json(&format!(
            "repos/{repository}/actions/artifacts?per_page=20&page={page}"
        ))?;
        let count = listing.artifacts.len();
        for artifact in listing.artifacts {
            let Some(attempt) = artifact.attempt(prefix) else {
                continue;
            };
            let Some(reference) = &artifact.run else {
                continue;
            };
            let run: Run = client.json(&format!(
                "repos/{repository}/actions/runs/{}/attempts/{attempt}",
                reference.id
            ))?;
            if !run.admitted(attempt) {
                continue;
            }
            let commit: Commit =
                client.json(&format!("repos/{repository}/git/commits/{}", run.head))?;
            if !trees.contains(&commit.tree.sha) {
                continue;
            }
            artifact.download(&client, &repository, target)?;
            eprintln!(
                "guard material: fetched accepted default-branch tree {} from Guard run {} attempt {attempt}",
                commit.tree.sha, reference.id
            );
            return Ok(true);
        }
        if count < 20 {
            break;
        }
    }
    Ok(false)
}

impl Artifact {
    fn attempt(&self, prefix: &str) -> Option<u64> {
        if self.expired || self.size == 0 || self.size > 10 * 1024 * 1024 * 1024 {
            return None;
        }
        self.digest
            .as_ref()?
            .strip_prefix("sha256:")
            .filter(|sha| super::context::hex(sha))?;
        self.name
            .strip_prefix(prefix)?
            .parse::<u64>()
            .ok()
            .filter(|attempt| *attempt != 0)
    }

    fn download(&self, client: &Client, repository: &str, target: &Path) -> Result<(), String> {
        crate::diagnostic::event(
            "material.candidate",
            || serde_json::json!({"artifact":self.id,"bytes":self.size,"run":self.run.as_ref().map(|run|run.id)}),
        );
        let parent = target.parent().ok_or("cache artifact has no parent")?;
        std::fs::create_dir_all(parent).map_err(|_| "cannot stage cache artifact")?;
        let archive =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| "cannot stage cache artifact")?;
        crate::diagnostic::measure("material.download", || {
            client.download(
                &format!("repos/{repository}/actions/artifacts/{}/zip", self.id),
                archive.path(),
                self.size,
            )
        })?;
        let sha = format!(
            "sha256:{}",
            crate::diagnostic::measure("material.digest", || digest(archive.path()))?
        );
        if self.digest.as_deref() != Some(&sha) {
            return Err("GitHub cache artifact digest mismatch".into());
        }
        let mut zip =
            zip::ZipArchive::new(archive.reopen().map_err(|_| "cannot read cache artifact")?)
                .map_err(|_| "cache artifact is not a ZIP")?;
        if zip.len() != 1 {
            return Err("cache artifact contains undeclared files".into());
        }
        let mut entry = zip
            .by_index(0)
            .map_err(|_| "cannot read cache artifact entry")?;
        if Path::new(entry.name()) != target.file_name().ok_or("cache parcel has no name")? {
            return Err("cache artifact parcel name mismatch".into());
        }
        if entry.size() > 10 * 1024 * 1024 * 1024 {
            return Err("cache artifact parcel exceeds its bound".into());
        }
        if let Ok(metadata) = std::fs::symlink_metadata(target)
            && (!metadata.is_file() || metadata.file_type().is_symlink())
        {
            return Err("cache artifact refuses a foreign parcel".into());
        }
        let mut file =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| "cannot stage cache parcel")?;
        std::io::copy(&mut entry, &mut file).map_err(|_| "cannot extract cache parcel")?;
        file.persist(target)
            .map_err(|_| "cannot publish cache parcel")?;
        Ok(())
    }
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
    use super::{Artifact, Run};

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

    #[test]
    fn selection() {
        let mut artifact = Artifact {
            id: 1,
            name: "owned-2".into(),
            size: 1024,
            digest: Some(format!("sha256:{}", "a".repeat(64))),
            expired: false,
            run: None,
        };
        assert_eq!(artifact.attempt("owned-"), Some(2));
        assert_eq!(artifact.attempt("foreign-"), None);
        artifact.expired = true;
        assert_eq!(artifact.attempt("owned-"), None);
        artifact.expired = false;
        artifact.name = "owned-0".into();
        assert_eq!(artifact.attempt("owned-"), None);
        artifact.name = "owned-2".into();
        artifact.digest = Some("sha256:unverified".into());
        assert_eq!(artifact.attempt("owned-"), None);
    }
}
