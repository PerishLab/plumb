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
        let change = change(root, &base, head)?;
        let body = body(issue, change, &delta(root, &base, head, &self.sources)?);
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

const CHANGE: &str = "Registered follow operation; manifests and lockfiles only.";
const MIGRATED: &str = "Registered follow operation; manifests and lockfiles only, plus the mechanical move of plumb.toml [release] product to [identity] ns.";

fn change(root: &std::path::Path, base: &str, head: &str) -> Result<&'static str, String> {
    let touched = workgit(
        root,
        &["diff", "--name-only", base, head, "--", "plumb.toml"],
    )?;
    if touched.is_empty() || !super::refresh::scope::migrated(root, base, head)? {
        return Ok(CHANGE);
    }
    Ok(MIGRATED)
}

fn body(issue: u64, change: &str, delta: &str) -> String {
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

impl Provider<'_> {
    pub(super) fn guard(&self, head: &str) -> Result<String, String> {
        self.settle(|settle| settle.guard(head))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn repository(manifest: &str) -> (tempfile::TempDir, String) {
        let root = tempfile::tempdir().unwrap();
        workgit(root.path(), &["init", "-q", "-b", "main"]).unwrap();
        workgit(root.path(), &["config", "user.name", "Fixture"]).unwrap();
        workgit(
            root.path(),
            &["config", "user.email", "fixture@example.invalid"],
        )
        .unwrap();
        std::fs::write(root.path().join("plumb.toml"), manifest).unwrap();
        std::fs::write(root.path().join("Cargo.lock"), "version = 4\n").unwrap();
        workgit(root.path(), &["add", "."]).unwrap();
        workgit(root.path(), &["commit", "-q", "-m", "base"]).unwrap();
        let base = workgit(root.path(), &["rev-parse", "HEAD"]).unwrap();
        (root, base)
    }

    fn head(root: &std::path::Path, message: &str) -> String {
        workgit(root, &["add", "-A"]).unwrap();
        workgit(root, &["commit", "-q", "-m", message]).unwrap();
        workgit(root, &["rev-parse", "HEAD"]).unwrap()
    }

    fn describe(root: &std::path::Path, base: &str, head: &str) -> String {
        body(
            520,
            change(root, base, head).unwrap(),
            "## Delta\nNo first-party package moved.\n",
        )
    }

    mod candidate {
        use super::*;

        #[test]
        fn lockfile() {
            let (root, base) = repository("[identity]\nns = \"probe\"\n");
            std::fs::write(
                root.path().join("Cargo.lock"),
                "version = 4\n\n[[package]]\nname = \"probe\"\nversion = \"0.2.0\"\n",
            )
            .unwrap();
            let head = head(root.path(), "follow");
            let body = describe(root.path(), &base, &head);
            assert!(body.contains(
                "## Change\nRegistered follow operation; manifests and lockfiles only.\n"
            ));
            assert!(!body.contains("[identity] ns"));
            assert!(body.contains("Refs #520."));
            assert!(body.contains("## Delta\nNo first-party package moved.\n"));
        }

        #[test]
        fn migration() {
            let (root, base) = repository("[release]\nproduct = \"probe\"\n");
            let before = std::fs::read_to_string(root.path().join("plumb.toml")).unwrap();
            let migrated = crate::shape::release::migration::rewrite(&before)
                .unwrap()
                .unwrap();
            std::fs::write(root.path().join("plumb.toml"), migrated).unwrap();
            let head = head(root.path(), "follow");
            let body = describe(root.path(), &base, &head);
            assert!(body.contains(
                "## Change\nRegistered follow operation; manifests and lockfiles only, plus the mechanical move of plumb.toml [release] product to [identity] ns.\n"
            ));
        }

        #[test]
        fn edit() {
            let (root, base) = repository("[identity]\nns = \"probe\"\n");
            std::fs::write(
                root.path().join("plumb.toml"),
                "[identity]\nns = \"probe\"\n\n[workflow]\n",
            )
            .unwrap();
            let head = head(root.path(), "follow");
            assert_eq!(change(root.path(), &base, &head).unwrap(), CHANGE);
        }
    }
}
