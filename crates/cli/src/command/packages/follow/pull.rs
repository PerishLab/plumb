use super::issue::text;
use super::provider::{Intent, Provider};
use serde_json::{Value, json};

impl Provider<'_> {
    pub(super) fn find(&self, issue: u64) -> Result<Option<Value>, String> {
        let branch = format!("auto/{issue}");
        let pulls = self.pages(&format!(
            "repos/{}/pulls?state=all&head={}:{}&base=main&per_page=100",
            self.repository,
            self.repository
                .split('/')
                .next()
                .ok_or("repository has no owner")?,
            branch
        ))?;
        if pulls.len() > 1 {
            return Err("multiple Auto pull requests require human judgment".into());
        }
        let pull = pulls.into_iter().next();
        if let Some(pull) = &pull {
            if (pull["head"]["ref"].as_str(), pull["base"]["ref"].as_str())
                != (Some(branch.as_str()), Some("main"))
            {
                return Err("Auto pull branches disagree with its registered issue".into());
            }
            if pull["head"]["repo"]["full_name"].as_str() != Some(self.repository) {
                return Err("Auto pull source repository disagrees".into());
            }
            if !pull["body"]
                .as_str()
                .is_some_and(|body| body.contains(&format!("Refs #{issue}.")))
            {
                return Err("Auto pull relationship disagrees with its registered issue".into());
            }
        }
        Ok(pull)
    }
}

pub(super) struct Publication<'a> {
    pub root: &'a std::path::Path,
    pub head: &'a str,
    pub previous: Option<&'a str>,
}

impl Provider<'_> {
    pub(super) fn publish(
        &self,
        issue: u64,
        publication: Publication<'_>,
    ) -> Result<Value, String> {
        let Publication {
            root,
            head,
            previous,
        } = publication;
        let branch = format!("auto/{issue}");
        let endpoint = format!("repos/{}/git/ref/heads/{branch}", self.repository);
        let expected = self.remote(issue)?.unwrap_or_default();
        if !expected.is_empty() && expected != head && previous != Some(expected.as_str()) {
            return Err(
                "Auto remote branch moved outside the recorded push; human judgment required"
                    .into(),
            );
        }
        if expected != head {
            let intent = Intent {
                schema: "plumb.auto-push/v1".into(),
                repository: self.repository.into(),
                issue,
                branch: branch.clone(),
                previous: (!expected.is_empty()).then_some(expected.clone()),
                head: head.into(),
                base: workgit(root, &["merge-base", "origin/main", head])?,
                tree: workgit(root, &["rev-parse", &format!("{head}^{{tree}}")])?,
            };
            self.intent(issue, &intent)?;
            let lease = format!("--force-with-lease=refs/heads/{branch}:{expected}");
            let spec = format!("{head}:refs/heads/{branch}");
            workgit(root, &["push", &lease, "origin", &spec])?;
        }
        let actual = self.api(&endpoint, None)?;
        if actual["object"]["sha"] != head {
            return Err("Auto push readback disagrees".into());
        }
        let base = workgit(root, &["merge-base", "origin/main", head])?;
        let body = body(issue, &delta(root, &base, head, &self.sources)?);
        if let Some(pull) = self.find(issue)? {
            if pull["body"].as_str() == Some(body.as_str()) {
                return Ok(pull);
            }
            let number = pull["number"].as_u64().ok_or("Auto pull has no number")?;
            let endpoint = format!("repos/{}/pulls/{number}", self.repository);
            return self.patch(&endpoint, &json!({"body":body}));
        }
        self.api(&format!("repos/{}/pulls", self.repository), Some(&json!({"title":"Follow the latest first-party packages", "head":branch, "base":"main", "body":body})))
    }

    pub(super) fn remote(&self, issue: u64) -> Result<Option<String>, String> {
        let branch = format!("auto/{issue}");
        let remote = self.pages(&format!(
            "repos/{}/git/matching-refs/heads/{branch}",
            self.repository
        ))?;
        let exact = remote
            .iter()
            .find(|reference| reference["ref"] == format!("refs/heads/{branch}"));
        exact
            .map(|reference| text(&reference["object"], "sha"))
            .transpose()
    }
}

fn body(issue: u64, delta: &str) -> String {
    format!(
        "Refs #{issue}.\n\n## Outcome\nFirst-party manifests and locks follow latest stable.\n\n## Change\nRegistered follow operation; manifests and lockfiles only, plus the mechanical move of plumb.toml [release] product to [identity] ns.\n\n{delta}\n## Verification\nRequired organization Guard must pass on this exact head before merge.\n\n## Boundary\nOnly the registered follow paths may change."
    )
}

fn delta(
    root: &std::path::Path,
    base: &str,
    head: &str,
    sources: &plumb::packages::Sources,
) -> Result<String, String> {
    let mut sides = Vec::new();
    for commit in [base, head] {
        let held = tempfile::tempdir().map_err(|error| error.to_string())?;
        for file in ["Cargo.lock", "pnpm-lock.yaml"] {
            if workgit(root, &["ls-tree", "--name-only", commit, "--", file])?.is_empty() {
                continue;
            }
            let text = workgit(root, &["show", &format!("{commit}:{file}")])?;
            std::fs::write(held.path().join(file), text).map_err(|error| error.to_string())?;
        }
        sides.push(plumb::packages::held(held.path())?);
    }
    Ok(plumb::packages::section(
        root, &sides[0], &sides[1], sources,
    ))
}

fn workgit(root: &std::path::Path, args: &[&str]) -> Result<String, String> {
    super::work::git(root, args)
}

impl Provider<'_> {
    pub(super) fn guard(&self, head: &str) -> Result<String, String> {
        self.settle(|settle| settle.guard(head))
    }
}
