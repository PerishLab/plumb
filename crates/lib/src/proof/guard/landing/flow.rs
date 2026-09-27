use super::repo::{Repository, Seed, line};
use super::{Refusal, refuse};
use crate::guard::Descriptor;
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

pub struct Draft {
    pub landing: Landing,
    pub story: Story,
    pub projection: String,
    pub tree: String,
    pub source: String,
    pub target: String,
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

    fn projection(&self) -> String {
        format!("land/{}", self.branch)
    }

    pub fn inspect(self, title: &str, body: &str) -> Result<Draft, Refusal> {
        self.landable()?;
        let story = self.describe(title, body)?;
        let upstream = self.upstream();
        let source = self.repo.revision("HEAD")?;
        let target = self.repo.revision(&upstream)?;
        let output = self.repo.git(&[
            "merge-tree",
            "--write-tree",
            "--no-messages",
            &upstream,
            "HEAD",
        ])?;
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
        let projection = self.projection();
        Ok(Draft {
            landing: self,
            story,
            projection,
            tree,
            source,
            target,
        })
    }

    fn landable(&self) -> Result<(), Refusal> {
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
        let upstream = self.upstream();
        if !self.repo.verified(&upstream) {
            return Err(refuse(
                "noupstream",
                format!("missing {upstream}; fetch or check the base branch name"),
            ));
        }
        let output = self.repo.git(&["diff", "--quiet", &upstream, "HEAD"])?;
        match output.status.code() {
            Some(0) => Err(refuse(
                "empty",
                format!("{} contributes no tree changes to {upstream}", self.branch),
            )),
            Some(1) => Ok(()),
            _ => Err(refuse("git", format!("cannot compare {upstream} and HEAD"))),
        }
    }

    fn describe(&self, title: &str, body: &str) -> Result<Story, Refusal> {
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
        Ok(Story {
            title,
            body: narrative(&body),
        })
    }
}

impl Draft {
    pub fn candidate(&self, proof: &Descriptor) -> Result<String, Refusal> {
        let token = proof.encode().map_err(|error| refuse("guard", error))?;
        let message = self.message(&token);
        let identity = self.identity()?;
        self.landing.repo.record(&Seed {
            tree: &self.tree,
            parents: &[&self.target],
            message: &message,
            identity: &identity,
        })
    }

    fn message(&self, proof: &str) -> String {
        let body = self.story.body.trim();
        let held = if body.is_empty() {
            String::new()
        } else {
            format!("\n\n{body}")
        };
        format!(
            "{}{held}\n\nLand-Source: {}@{}\n{} {proof}\n",
            self.story.title.trim(),
            self.landing.branch,
            self.source,
            crate::guard::TRAILER
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
            let value = self.landing.repo.text(
                &["show", "-s", &format!("--format={shape}"), "HEAD"],
                "git",
                "cannot read commit identity",
            )?;
            held.insert(name.to_string(), value);
        }
        Ok(held)
    }
}

fn below(body: &str, title: &str) -> String {
    body.strip_prefix(title)
        .map(|rest| rest.trim_start_matches('\n').to_string())
        .unwrap_or_else(|| body.to_string())
}

fn narrative(body: &str) -> String {
    body.lines()
        .filter(|line| {
            !line.starts_with(crate::guard::TRAILER) && !line.starts_with("Land-Source:")
        })
        .collect::<Vec<_>>()
        .join("\n")
}
