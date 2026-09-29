use crate::landing::{Refusal, refuse};
use std::path::Path;
use std::process::Command;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Squash {
    pub candidate: String,
    pub subject: String,
    pub body: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Landed {
    pub head: String,
    pub parent: String,
    pub tree: String,
}

impl Squash {
    pub fn read(root: &Path, candidate: &str) -> Result<Self, Refusal> {
        let message = git(root, &["log", "-1", "--format=%B", candidate])?;
        let (subject, body) = message.split_once('\n').unwrap_or((&message, ""));
        if subject.trim().is_empty() {
            return Err(refuse(
                "squash",
                format!("candidate {candidate} has no subject to squash under"),
            ));
        }
        Ok(Self {
            candidate: candidate.to_string(),
            subject: subject.to_string(),
            body: body.trim_start_matches('\n').to_string(),
        })
    }

    pub fn arguments(&self, repository: &str, number: u64) -> Vec<String> {
        [
            "pr",
            "merge",
            &number.to_string(),
            "-R",
            repository,
            "--squash",
            "--match-head-commit",
            &self.candidate,
            "--subject",
            &self.subject,
            "--body",
            &self.body,
        ]
        .map(str::to_string)
        .to_vec()
    }
}

pub fn landed(root: &Path, candidate: &str, head: &str) -> Result<Landed, Refusal> {
    let parent = |commit: &str| git(root, &["rev-parse", "--verify", &format!("{commit}^")]);
    let tree = |commit: &str| {
        git(
            root,
            &["rev-parse", "--verify", &format!("{commit}^{{tree}}")],
        )
    };
    let head = &git(
        root,
        &["rev-parse", "--verify", &format!("{head}^{{commit}}")],
    )?;
    let proved = parent(candidate)?;
    let held = parent(head)?;
    if held != proved {
        return Err(refuse(
            "moved",
            format!(
                "the base moved under the merge: {head} sits on {held}, but Guard proved candidate {candidate} on {proved}; main now holds a tree no Guard proved, and a merge cannot be undone, so guard what main holds and land any repair as a new change"
            ),
        ));
    }
    let sealed = tree(candidate)?;
    let merged = tree(head)?;
    if merged != sealed {
        return Err(refuse(
            "tree",
            format!(
                "squash {head} holds tree {merged}, not the tree {sealed} Guard proved for candidate {candidate} on the same base {proved}"
            ),
        ));
    }
    let lost = |error: String| {
        refuse(
            "proof",
            format!("squash {head} lost candidate {candidate}'s Guard proof: {error}"),
        )
    };
    let carried = crate::guard::commit(root, head).map_err(lost)?;
    let expected = crate::guard::commit(root, candidate).map_err(lost)?;
    if carried != expected {
        return Err(lost("it carries a different proof".to_string()));
    }
    Ok(Landed {
        head: head.to_string(),
        parent: held,
        tree: merged,
    })
}

fn git(root: &Path, args: &[&str]) -> Result<String, Refusal> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| refuse("git", format!("cannot run git: {error}")))?;
    if !output.status.success() {
        return Err(refuse(
            "git",
            format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string())
}
