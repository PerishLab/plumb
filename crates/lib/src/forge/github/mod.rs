mod delivery;

use self::delivery as github;
pub use crate::config::github::{client, diagnostic};
use crate::delivery::{Guard, Patience, Squash};
pub use delivery::{guard, merge};
use runseal::github::{
    Compare, Number, Proposal, Signal, State, Target, Verdict, lifecycle::Selection,
};
use std::path::Path;

pub struct Remote {
    pub owner: String,
    pub repo: String,
}

pub struct Pull {
    pub node: String,
    pub number: u64,
    pub url: String,
    pub head: String,
    pub title: String,
    pub body: String,
}

pub fn remote(root: &Path) -> Result<Remote, String> {
    let output = crate::config::detached("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "read origin failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse(String::from_utf8_lossy(&output.stdout).trim())
}

fn parse(raw: &str) -> Result<Remote, String> {
    let path = raw
        .strip_prefix("git@github.com:")
        .or_else(|| raw.strip_prefix("ssh://git@github.com/"))
        .or_else(|| raw.strip_prefix("https://github.com/"))
        .ok_or_else(|| {
            format!("origin {raw} is not GitHub; Forgejo is archived and main lives on GitHub")
        })?;
    let mut parts = path.trim_end_matches(".git").split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(owner), Some(repo), None) if !owner.is_empty() && !repo.is_empty() => Ok(Remote {
            owner: owner.to_string(),
            repo: repo.to_string(),
        }),
        _ => Err(format!("cannot derive owner/repo from origin {raw}")),
    }
}

pub struct Client<'a> {
    scope: &'a Remote,
    github: runseal::github::Client,
}

impl<'a> Client<'a> {
    pub fn new(scope: &'a Remote) -> Result<Self, String> {
        Ok(Self {
            scope,
            github: client()?,
        })
    }

    pub fn opened(&self, base: &str, head: &str) -> Result<Option<Pull>, String> {
        let selection = Selection::new(
            head,
            Some(base),
            State::Open,
            Number::new(1, 2).map_err(diagnostic)?,
        )
        .map_err(diagnostic)?;
        let page = self
            .github
            .listing(&self.scope.owner, &self.scope.repo, &selection)
            .map_err(diagnostic)?
            .value;
        if page.next.is_some() || page.rows.len() > 1 {
            return Err("multiple open pulls match the landing branch".into());
        }
        Ok(page.rows.into_iter().next().map(Pull::from))
    }

    pub fn raise(&self, base: &str, head: &str, title: &str, body: &str) -> Result<Pull, String> {
        let compare = Compare::new(head, base).map_err(diagnostic)?;
        let proposal = Proposal::new(compare, title, Some(body)).map_err(diagnostic)?;
        let pull = self
            .github
            .propose(&self.scope.owner, &self.scope.repo, &proposal)
            .map_err(diagnostic)?
            .value;
        Ok(Pull::from(pull))
    }

    pub fn mark(&self, commit: &str, context: &str, description: &str) -> Result<(), String> {
        let input =
            Signal::new(Verdict::Success, context, Some(description), None).map_err(diagnostic)?;
        self.github
            .mark(&self.scope.owner, &self.scope.repo, commit, &input)
            .map_err(diagnostic)?;
        Ok(())
    }

    pub fn settle(&self, number: u64, squash: &Squash) -> Result<(), String> {
        let (owner, repo) = (&self.scope.owner, &self.scope.repo);
        let number = u32::try_from(number).map_err(|_| "invalid GitHub pull number")?;
        let target = Target::new(owner, repo, number).map_err(diagnostic)?;
        let guard = Guard {
            head: &squash.candidate,
            noun: "land",
        };
        let patience = Patience::MERGE;
        Guard::wait(patience.pause, || {
            github::guard(&self.github, owner, repo, &guard)
        })?;
        github::merge(&target, &self.github, squash, "land")
    }
}

impl From<runseal::github::Pull> for Pull {
    fn from(pull: runseal::github::Pull) -> Self {
        Self {
            node: pull.node,
            number: pull.number.into(),
            url: pull.url,
            head: pull.head.sha,
            title: pull.title,
            body: pull.body.unwrap_or_default(),
        }
    }
}

#[cfg(all(test, unix))]
mod fixture;

#[cfg(test)]
mod tests;
