use std::path::Path;
use std::process::{Command, Output};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Standing {
    Ancestor,
    Contained,
    Lacking(Vec<String>),
    Conflicting(Vec<String>),
}

impl Standing {
    pub fn home(&self) -> bool {
        matches!(self, Self::Ancestor | Self::Contained)
    }
}

pub fn contained(root: &Path, stable: &str, main: &str) -> Result<bool, String> {
    standing(root, stable, main).map(|held| held.home())
}

pub fn standing(root: &Path, stable: &str, main: &str) -> Result<Standing, String> {
    let ancestry = git(root, &["merge-base", "--is-ancestor", stable, main])?;
    match ancestry.status.code() {
        Some(0) => return Ok(Standing::Ancestor),
        Some(1) => {}
        _ => {
            return Err(failed(
                &ancestry,
                format!("cannot compare {stable} with {main}"),
            ));
        }
    }
    let tree = text(
        git(
            root,
            &["rev-parse", "--verify", &format!("{main}^{{tree}}")],
        )?,
        format!("cannot resolve {main}'s tree"),
    )?;
    let merged = git(
        root,
        &[
            "merge-tree",
            "--write-tree",
            "--name-only",
            "--no-messages",
            main,
            stable,
        ],
    )?;
    let listed = String::from_utf8_lossy(&merged.stdout).to_string();
    let mut lines = listed
        .lines()
        .map(str::trim)
        .filter(|held| !held.is_empty());
    let written = lines.next().unwrap_or_default().to_string();
    match merged.status.code() {
        Some(0) if written == tree => Ok(Standing::Contained),
        Some(0) => {
            let paths = text(
                git(root, &["diff", "--name-only", &tree, &written])?,
                format!("cannot compare {main} with its merge of {stable}"),
            )?;
            Ok(Standing::Lacking(
                paths.lines().map(str::to_string).collect(),
            ))
        }
        Some(1) => Ok(Standing::Conflicting(lines.map(str::to_string).collect())),
        _ => Err(failed(
            &merged,
            format!("cannot merge {stable} into {main}"),
        )),
    }
}

fn git(root: &Path, args: &[&str]) -> Result<Output, String> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))
}

fn text(output: Output, whose: String) -> Result<String, String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(failed(&output, whose))
    }
}

fn failed(output: &Output, whose: String) -> String {
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if detail.is_empty() {
        whose
    } else {
        format!("{whose}: {detail}")
    }
}
