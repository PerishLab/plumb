use super::provider::{Intent, Provider, Source, encode};
use crate::command::diagnostic;
use runseal::github::{Compare, Proposal, Reference, Target, lifecycle::Update};
use serde_json::Value;
#[cfg(all(test, unix))]
use serde_json::json;

impl Provider<'_> {
    pub(super) fn find(&self, issue: u64) -> Result<Option<Value>, String> {
        let branch = format!("auto/{issue}");
        let mut pulls = self.listing(&branch, Some("main"))?;
        if pulls.len() > 1 {
            return Err("multiple Auto pull requests require human judgment".into());
        }
        let pull = pulls.pop();
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
    pub(super) fn target(&self, number: u64) -> Result<Target, String> {
        let (owner, name) = self.coordinate()?;
        let number = u32::try_from(number).map_err(|_| "invalid GitHub Issue or pull number")?;
        Target::new(owner, name, number).map_err(diagnostic)
    }

    pub(super) fn pull(&self, number: u64) -> Result<Value, String> {
        match &self.source {
            Source::Native(client) => encode(
                client
                    .pull(&self.target(number)?)
                    .map_err(diagnostic)?
                    .value,
            ),
            #[cfg(all(test, unix))]
            Source::Fixture => self.api(&format!("repos/{}/pulls/{number}", self.repository), None),
        }
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
        if self.tip(&branch)? != head {
            return Err("Auto push readback disagrees".into());
        }
        let base = workgit(root, &["merge-base", "origin/main", head])?;
        let change = change(root, &base, head)?;
        let body = body(issue, change, &delta(root, &base, head, &self.sources)?);
        if let Some(pull) = self.find(issue)? {
            if pull["body"].as_str() == Some(body.as_str()) {
                return Ok(pull);
            }
            let number = pull["number"].as_u64().ok_or("Auto pull has no number")?;
            return self.revise(number, &Update::Body(body));
        }
        self.propose(&branch, head, &body)
    }

    fn propose(&self, branch: &str, head: &str, body: &str) -> Result<Value, String> {
        let title = "Follow the latest first-party packages";
        match &self.source {
            Source::Native(client) => {
                let (owner, name) = self.coordinate()?;
                let compare = Compare::new(branch, "main").map_err(diagnostic)?;
                let input = Proposal::new(compare, title, Some(body)).map_err(diagnostic)?;
                let pull = client
                    .propose(owner, name, &input)
                    .map_err(diagnostic)?
                    .value;
                if pull.head.sha != head || pull.state != "open" {
                    return Err("Auto pull creation did not confirm the exact open head; reread before retry".into());
                }
                encode(pull)
            }
            #[cfg(all(test, unix))]
            Source::Fixture => self.api(
                &format!("repos/{}/pulls", self.repository),
                Some(&json!({
                    "title":title, "head":branch, "base":"main", "body":body
                })),
            ),
        }
    }

    fn tip(&self, branch: &str) -> Result<String, String> {
        match &self.source {
            Source::Native(client) => {
                let (owner, name) = self.coordinate()?;
                let reference = Reference::Branch(branch.into());
                Ok(client
                    .reference(owner, name, &reference)
                    .map_err(diagnostic)?
                    .value
                    .object
                    .sha)
            }
            #[cfg(all(test, unix))]
            Source::Fixture => {
                let value = self.api(
                    &format!("repos/{}/git/ref/heads/{branch}", self.repository),
                    None,
                )?;
                super::issue::text(&value["object"], "sha")
            }
        }
    }

    pub(super) fn remote(&self, issue: u64) -> Result<Option<String>, String> {
        let branch = format!("auto/{issue}");
        let exact = format!("refs/heads/{branch}");
        match &self.source {
            Source::Native(client) => {
                let (owner, name) = self.coordinate()?;
                let refs = client
                    .matching(owner, name, &branch)
                    .map_err(diagnostic)?
                    .value;
                Ok(refs
                    .into_iter()
                    .find(|row| row.reference == exact)
                    .map(|row| row.object.sha))
            }
            #[cfg(all(test, unix))]
            Source::Fixture => {
                let refs = self.pages(&format!(
                    "repos/{}/git/matching-refs/heads/{branch}",
                    self.repository
                ))?;
                refs.iter()
                    .find(|row| row["ref"] == exact)
                    .map(|row| super::issue::text(&row["object"], "sha"))
                    .transpose()
            }
        }
    }
}

const CHANGE: &str = "Registered follow operation; manifests and lockfiles only.";
const MIGRATED: &str = "Registered follow operation; manifests and lockfiles only, plus the mechanical move of plumb.toml [release] product to [identity] ns.";

pub(super) fn change(
    root: &std::path::Path,
    base: &str,
    head: &str,
) -> Result<&'static str, String> {
    let touched = workgit(
        root,
        &["diff", "--name-only", base, head, "--", "plumb.toml"],
    )?;
    if touched.is_empty() || !super::refresh::scope::migrated(root, base, head)? {
        return Ok(CHANGE);
    }
    Ok(MIGRATED)
}

pub(super) fn body(issue: u64, change: &str, delta: &str) -> String {
    format!(
        "Refs #{issue}.\n\n## Outcome\nFirst-party manifests and locks follow latest stable.\n\n## Change\n{change}\n\n{delta}\n## Verification\nRequired organization Guard must pass on this exact head before merge.\n\n## Boundary\nOnly the registered follow paths may change."
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
