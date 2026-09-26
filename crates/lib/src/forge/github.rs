use serde_json::Value;
use std::path::Path;
use std::process::Command;

pub mod issue;

pub use issue::Issue;

pub struct Remote {
    pub owner: String,
    pub repo: String,
}

pub struct Pull {
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
}

impl Client {
    pub fn new(remote: &Remote) -> Self {
        Self {
            seat: format!("{}/{}", remote.owner, remote.repo),
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

    pub fn issue(&self, number: u64) -> Result<Issue, String> {
        let number = number.to_string();
        let viewed = self.gh(&[
            "issue",
            "view",
            &number,
            "-R",
            &self.seat,
            "--json",
            issue::FIELDS,
        ])?;
        let value: Value = serde_json::from_str(&viewed)
            .map_err(|error| format!("gh issue view did not answer JSON: {error}"))?;
        issue::parse(&value).ok_or_else(|| format!("issue {number} has no stable identity"))
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

    pub fn settle(&self, number: u64, head: &str) -> Result<(), String> {
        let number = number.to_string();
        let mut last = String::new();
        for turn in 1..=6 {
            let merged = self.gh(&[
                "pr",
                "merge",
                &number,
                "-R",
                &self.seat,
                "--merge",
                "--match-head-commit",
                head,
            ]);
            match merged {
                Ok(_) => return Ok(()),
                Err(error) if pending(&error) => last = error,
                Err(error) => return Err(error),
            }
            std::thread::sleep(std::time::Duration::from_secs(turn));
        }
        Err(last)
    }

    fn gh(&self, args: &[&str]) -> Result<String, String> {
        let output = Command::new("gh")
            .args(args)
            .output()
            .map_err(|error| format!("cannot run gh: {error}"))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
        let named = args
            .iter()
            .take_while(|arg| !arg.starts_with('-'))
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        Err(format!(
            "gh {named} on {} failed: {}",
            self.seat,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

const FIELDS: &str = "number,url,headRefOid,title,body";
fn pull(value: &Value) -> Option<Pull> {
    let text = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    Some(Pull {
        number: value.get("number")?.as_u64()?,
        url: text("url"),
        head: text("headRefOid"),
        title: text("title"),
        body: text("body"),
    })
}

fn pending(error: &str) -> bool {
    ["not mergeable", "is expected", "mergeability"]
        .iter()
        .any(|held| error.contains(held))
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn remotes() {
        for raw in [
            "git@github.com:PerishLab/plumb.git",
            "ssh://git@github.com/PerishLab/plumb.git",
            "https://github.com/PerishLab/plumb",
        ] {
            let held = parse(raw).expect(raw);
            assert_eq!(
                (held.owner.as_str(), held.repo.as_str()),
                ("PerishLab", "plumb")
            );
        }
        let refused = parse("ssh://git@git.perish.top/PerishLab/plumb.git").err();
        assert!(refused.is_some_and(|error| error.contains("Forgejo is archived")));
    }
}
