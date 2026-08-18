use super::repo::{Repository, Seed, line, success};
use super::{Refusal, refuse};
use std::collections::BTreeMap;

pub struct Landing {
    pub repo: Repository,
    pub base: String,
    pub branch: String,
}

pub struct Story {
    pub title: String,
    pub body: String,
}

pub struct Candidate {
    pub projection: String,
    pub head: String,
    pub source: String,
    pub base: String,
}

impl Landing {
    pub fn open(root: &std::path::Path, base: &str) -> Result<Self, Refusal> {
        let repo = Repository::open(root)?;
        let branch = repo.branch()?;
        Ok(Self {
            repo,
            base: base.to_string(),
            branch,
        })
    }

    pub fn upstream(&self) -> String {
        format!("origin/{}", self.base)
    }

    pub fn projection(&self) -> String {
        format!("land/{}", self.branch)
    }

    pub fn landable(&self, fetch: bool) -> Result<(), Refusal> {
        if self.branch == self.base || self.branch == "main" || self.branch == "master" {
            return Err(refuse(
                "onbase",
                format!("land must run on a topic branch, not {}", self.branch),
            ));
        }
        if self.branch.starts_with("release/") || self.base.starts_with("release/") {
            return Err(refuse(
                "release",
                "release lines settle through the stable lane, not through land",
            ));
        }
        self.repo.clean()?;
        if fetch {
            self.repo.fetch()?;
        }
        let upstream = self.upstream();
        if !self.repo.verified(&upstream) {
            return Err(refuse(
                "noupstream",
                format!("missing {upstream}; fetch or check the base branch name"),
            ));
        }
        self.differs(&upstream)
    }

    fn differs(&self, upstream: &str) -> Result<(), Refusal> {
        let output = self.repo.git(&["diff", "--quiet", upstream, "HEAD"])?;
        match output.status.code() {
            Some(0) => Err(refuse(
                "empty",
                format!("{} contributes no tree changes to {upstream}", self.branch),
            )),
            Some(1) => Ok(()),
            _ => Err(refuse("git", format!("cannot compare {upstream} and HEAD"))),
        }
    }

    pub fn describe(&self, title: &str, body: &str) -> Result<Story, Refusal> {
        let range = format!("{}..HEAD", self.upstream());
        let title = if title.is_empty() {
            let subjects = self.repo.text(
                &["log", "--reverse", "--format=%s", &range],
                "git",
                "cannot read commit subjects",
            )?;
            subjects
                .lines()
                .map(str::trim)
                .find(|held| !held.is_empty())
                .unwrap_or(&self.branch)
                .to_string()
        } else {
            title.to_string()
        };
        let body = if body.is_empty() {
            let held = self.repo.text(
                &["log", "--reverse", "--format=%B", &range],
                "git",
                "cannot read commit bodies",
            )?;
            below(&held, &title)
        } else {
            body.to_string()
        };
        Ok(Story { title, body })
    }

    pub fn derive(&self, story: &Story) -> Result<Candidate, Refusal> {
        let upstream = self.upstream();
        let source = self.repo.revision("HEAD")?;
        let target = self.repo.revision(&upstream)?;
        let merged = [
            "merge-tree",
            "--write-tree",
            "--no-messages",
            &upstream,
            "HEAD",
        ];
        let output = self.repo.git(&merged)?;
        if !output.status.success() {
            return Err(refuse(
                "conflict",
                format!(
                    "{} conflicts with {upstream}; resolve it on the source line",
                    self.branch
                ),
            ));
        }
        let tree = line(output.stdout)?;
        if tree == self.repo.revision(&format!("{target}^{{tree}}"))? {
            return Err(refuse(
                "empty",
                format!("{} contributes no tree changes to {upstream}", self.branch),
            ));
        }
        let message = self.message(story, &source);
        let identity = self.identity()?;
        let head = self.repo.record(&Seed {
            tree: &tree,
            parent: &target,
            message: &message,
            identity: &identity,
        })?;
        Ok(Candidate {
            projection: self.projection(),
            head,
            source,
            base: target,
        })
    }

    fn message(&self, story: &Story, source: &str) -> String {
        let body = story.body.trim();
        let held = if body.is_empty() {
            String::new()
        } else {
            format!("\n\n{body}")
        };
        format!(
            "{}{held}\n\nLand-Source: {}@{source}\n",
            story.title.trim(),
            self.branch
        )
    }

    fn identity(&self) -> Result<BTreeMap<String, String>, Refusal> {
        let mut held = BTreeMap::new();
        for (name, shape) in [
            ("GIT_AUTHOR_NAME", "%an"),
            ("GIT_AUTHOR_EMAIL", "%ae"),
            ("GIT_AUTHOR_DATE", "%aI"),
            ("GIT_COMMITTER_NAME", "%cn"),
            ("GIT_COMMITTER_EMAIL", "%ce"),
            ("GIT_COMMITTER_DATE", "%cI"),
        ] {
            let value = self.repo.text(
                &["show", "-s", &format!("--format={shape}"), "HEAD"],
                "git",
                "cannot read commit identity",
            )?;
            held.insert(name.to_string(), value);
        }
        Ok(held)
    }

    pub fn push(&self, branch: &str, source: &str, upstream: bool) -> Result<(), Refusal> {
        let reference = format!("refs/heads/{branch}");
        let tracking = format!("refs/remotes/origin/{branch}");
        let expected = if self.repo.verified(&tracking) {
            self.repo.revision(&tracking)?
        } else {
            String::new()
        };
        let lease = format!("--force-with-lease={reference}:{expected}");
        let spec = format!("{source}:{reference}");
        let mut args = vec!["push"];
        if upstream {
            args.push("-u");
        }
        args.extend_from_slice(&[&lease, "origin", &spec]);
        let output = self.repo.git(&args)?;
        success(output, "push", format!("cannot push {branch}")).map(|_| ())
    }

    pub fn settled(&self, candidate: &Candidate) -> Result<(), Refusal> {
        if self.repo.revision("HEAD")? != candidate.source {
            return Err(refuse(
                "sourcechanged",
                format!(
                    "{} changed while its projection was guarded; run land again",
                    self.branch
                ),
            ));
        }
        self.repo.fetch()?;
        let upstream = self.upstream();
        if self.repo.revision(&upstream)? != candidate.base {
            return Err(refuse(
                "stale",
                format!("{upstream} advanced while its projection was guarded; run land again"),
            ));
        }
        Ok(())
    }

    pub fn sync(&self) -> Result<Option<std::path::PathBuf>, Refusal> {
        let Some(seat) = self.repo.seat(&self.base)? else {
            return Ok(None);
        };
        let held = Repository::open(&seat)?;
        let output = held.git(&["pull", "--ff-only", "origin", &self.base])?;
        success(
            output,
            "sync",
            format!("cannot sync {} at {}", self.base, seat.display()),
        )?;
        Ok(Some(seat))
    }
}

fn below(body: &str, title: &str) -> String {
    body.strip_prefix(title)
        .map(|rest| rest.trim_start_matches('\n').to_string())
        .unwrap_or_else(|| body.to_string())
}
