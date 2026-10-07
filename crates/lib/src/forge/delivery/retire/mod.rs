use super::squash::{Landed, landed};
use crate::landing::Refusal;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pushed {
    pub branch: String,
    pub head: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Retired {
    pub deleted: Vec<String>,
    pub kept: Vec<Kept>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Kept {
    pub branch: String,
    pub seat: String,
    pub reason: String,
}

pub struct Delivered<'a> {
    pub root: &'a Path,
    pub remote: &'a str,
    pub candidate: &'a str,
    pub head: &'a str,
}

pub fn retired(delivered: &Delivered<'_>, pushed: &[Pushed]) -> Result<(Landed, Retired), Refusal> {
    let held = landed(delivered.root, delivered.candidate, delivered.head)?;
    let retired = held.retire(delivered.root, delivered.remote, pushed);
    Ok((held, retired))
}

impl Landed {
    pub fn retire(&self, root: &Path, remote: &str, pushed: &[Pushed]) -> Retired {
        let mut retired = Retired::default();
        for branch in pushed {
            Seat {
                root,
                remote,
                pushed: branch,
            }
            .retire(&mut retired);
        }
        retired
    }
}

struct Seat<'a> {
    root: &'a Path,
    remote: &'a str,
    pushed: &'a Pushed,
}

impl Seat<'_> {
    fn retire(&self, retired: &mut Retired) {
        let mut kept = false;
        for (seat, outcome) in [(self.remote, self.origin()), ("local", self.local())] {
            if let Err(reason) = outcome {
                kept = true;
                retired.kept.push(Kept {
                    branch: self.pushed.branch.clone(),
                    seat: seat.to_string(),
                    reason,
                });
            }
        }
        if !kept {
            retired.deleted.push(self.pushed.branch.clone());
        }
    }

    fn reference(&self) -> String {
        format!("refs/heads/{}", self.pushed.branch)
    }

    fn origin(&self) -> Result<(), String> {
        let reference = self.reference();
        let listed = text(self.git(&["ls-remote", "--heads", self.remote, &reference])?)?;
        let Some(held) = listed.split_whitespace().next() else {
            return Ok(());
        };
        if held != self.pushed.head {
            return Err(format!(
                "it holds {held}, not the pushed {}",
                self.pushed.head
            ));
        }
        let lease = format!("--force-with-lease={reference}:{}", self.pushed.head);
        text(self.git(&["push", &lease, self.remote, &format!(":{reference}")])?).map(|_| ())
    }

    fn local(&self) -> Result<(), String> {
        let reference = self.reference();
        let probe = self.git(&["show-ref", "--verify", "--hash", &reference])?;
        if !probe.status.success() {
            return Ok(());
        }
        let held = text(probe)?;
        if held != self.pushed.head {
            return Err(format!("it moved to {held} after the push"));
        }
        if let Some(path) = self.checkout(&reference)? {
            if !same(&path, self.root) {
                return Err(format!("it is checked out at {}", path.display()));
            }
            text(self.git(&["checkout", "-q", "--detach"])?)?;
        }
        text(self.git(&["update-ref", "-d", &reference, &self.pushed.head])?).map(|_| ())
    }

    fn checkout(&self, reference: &str) -> Result<Option<PathBuf>, String> {
        let listed = text(self.git(&["worktree", "list", "--porcelain"])?)?;
        let wanted = format!("branch {reference}");
        let mut seen = None;
        for line in listed.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                seen = Some(PathBuf::from(path));
            }
            if line == wanted {
                return Ok(seen);
            }
        }
        Ok(None)
    }

    fn git(&self, args: &[&str]) -> Result<Output, String> {
        Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(args)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))
    }
}

fn same(left: &Path, right: &Path) -> bool {
    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn text(output: Output) -> Result<String, String> {
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
}
