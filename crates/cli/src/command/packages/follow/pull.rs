use super::issue::text;
use super::provider::Provider;
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
            let lease = format!("--force-with-lease=refs/heads/{branch}:{expected}");
            let spec = format!("{head}:refs/heads/{branch}");
            workgit(root, &["push", &lease, "origin", &spec])?;
        }
        let actual = self.api(&endpoint, None)?;
        if actual["object"]["sha"] != head {
            return Err("Auto push readback disagrees".into());
        }
        if let Some(pull) = self.find(issue)? {
            return Ok(pull);
        }
        self.api(&format!("repos/{}/pulls", self.repository), Some(&json!({"title":"Follow the latest first-party packages", "head":branch, "base":"main", "body":format!("Refs #{issue}.\n\n## Outcome\nFirst-party manifests and locks follow latest stable.\n\n## Change\nRegistered follow operation; manifests and lockfiles only.\n\n## Verification\nRequired organization Guard must pass on this exact head before merge.\n\n## Boundary\nOnly the registered follow paths may change." )})))
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

fn workgit(root: &std::path::Path, args: &[&str]) -> Result<String, String> {
    super::work::git(root, args)
}

impl Provider<'_> {
    pub(super) fn guard(&self, head: &str) -> Result<String, String> {
        let runs = self.pages(&format!(
            "repos/{}/commits/{head}/check-runs?per_page=100",
            self.repository
        ))?;
        let mut matches = Vec::new();
        for page in runs {
            for run in page["check_runs"]
                .as_array()
                .ok_or("check-run response is unreadable")?
            {
                if run["name"] == "Guard"
                    && run["app"]["slug"] == "github-actions"
                    && run["head_sha"] == head
                {
                    matches.push(run.clone());
                }
            }
        }
        if matches.is_empty() {
            return Err("organization Guard is pending; no run on the exact Auto head".into());
        }
        if matches.len() != 1 {
            return Err(
                "exact Auto head has multiple organization Guard runs; human judgment required"
                    .into(),
            );
        }
        let run = &matches[0];
        if run["status"] != "completed" {
            return Err("organization Guard is pending; rerun follow after it completes".into());
        }
        if run["conclusion"] != "success" {
            return Err(format!("organization Guard failed: {}", run["conclusion"]));
        }
        let url = text(run, "details_url")?;
        let id = url
            .split("/actions/runs/")
            .nth(1)
            .and_then(|tail| tail.split('/').next())
            .filter(|id| id.parse::<u64>().is_ok())
            .ok_or("Guard check has no workflow run identity")?;
        let workflow = self.api(
            &format!("repos/{}/actions/runs/{id}", self.repository),
            None,
        )?;
        if workflow["head_sha"] != head
            || workflow["path"] != ".github/workflows/guard.yml"
            || !matches!(
                workflow["event"].as_str(),
                Some("pull_request" | "merge_group")
            )
        {
            return Err("Guard check is not the organization workflow on this exact head".into());
        }
        Ok(url)
    }
}
