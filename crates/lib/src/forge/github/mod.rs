use crate::delivery::{Patience, Settle, Squash};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    let output = Command::new("git")
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

pub struct Client {
    seat: String,
    command: PathBuf,
    patience: Patience,
}

impl Client {
    pub fn new(remote: &Remote) -> Self {
        Self {
            seat: format!("{}/{}", remote.owner, remote.repo),
            command: PathBuf::from("gh"),
            patience: Patience::MERGE,
        }
    }

    pub fn opened(&self, base: &str, head: &str) -> Result<Option<Pull>, String> {
        let listed = self.gh(&[
            "pr", "list", "-R", &self.seat, "--state", "open", "--base", base, "--head", head,
            "--json", FIELDS,
        ])?;
        let value: Value = serde_json::from_str(&listed)
            .map_err(|error| format!("gh pr list did not answer JSON: {error}"))?;
        Ok(value
            .as_array()
            .and_then(|held| held.first())
            .and_then(pull))
    }

    pub fn raise(&self, base: &str, head: &str, title: &str, body: &str) -> Result<Pull, String> {
        let url = self.gh(&[
            "pr", "create", "-R", &self.seat, "--base", base, "--head", head, "--title", title,
            "--body", body,
        ])?;
        let url = url
            .lines()
            .rev()
            .find(|line| line.contains("/pull/"))
            .ok_or_else(|| format!("gh pr create named no pull: {url}"))?
            .trim()
            .to_string();
        let viewed = self.gh(&["pr", "view", &url, "-R", &self.seat, "--json", FIELDS])?;
        let value: Value = serde_json::from_str(&viewed)
            .map_err(|error| format!("gh pr view did not answer JSON: {error}"))?;
        pull(&value).ok_or_else(|| format!("{url} has no number"))
    }

    pub fn mark(&self, commit: &str, context: &str, description: &str) -> Result<(), String> {
        self.gh(&[
            "api",
            "-X",
            "POST",
            &format!("repos/{}/statuses/{commit}", self.seat),
            "-f",
            "state=success",
            "-f",
            &format!("context={context}"),
            "-f",
            &format!("description={description}"),
        ])
        .map(|_| ())
    }

    pub fn settle(&self, number: u64, squash: &Squash) -> Result<(), String> {
        let gh = |args: &[String]| self.run(args);
        let settle = Settle {
            repository: &self.seat,
            noun: "land",
            patience: self.patience,
            gh: &gh,
        };
        settle.wait(&squash.candidate)?;
        settle.merge(number, squash)
    }

    fn gh(&self, args: &[&str]) -> Result<String, String> {
        let args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        self.run(&args)
            .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string())
    }

    fn run(&self, args: &[String]) -> Result<Vec<u8>, String> {
        let output = Command::new(&self.command)
            .args(args)
            .output()
            .map_err(|error| format!("cannot run gh: {error}"))?;
        if output.status.success() {
            return Ok(output.stdout);
        }
        let named = args
            .iter()
            .take_while(|arg| !arg.starts_with('-'))
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" ");
        Err(format!(
            "gh {named} on {} failed: {}",
            self.seat,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

const FIELDS: &str = "id,number,url,headRefOid,title,body";
fn pull(value: &Value) -> Option<Pull> {
    let text = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    Some(Pull {
        node: text("id"),
        number: value.get("number")?.as_u64()?,
        url: text("url"),
        head: text("headRefOid"),
        title: text("title"),
        body: text("body"),
    })
}

#[cfg(test)]
mod tests;
