use crate::command::doctor::dependency;
use plumb::datum::{self, Datum};
use std::path::Path;

pub struct Cut<'a> {
    pub root: &'a Path,
    pub version: &'a str,
    pub head: &'a str,
}

pub struct Seat<'a>(pub &'a Path);

pub struct Record {
    pub head: String,
    pub report: String,
}

pub fn plan(version: &str) -> String {
    format!("record the {version} datum in Git commit metadata")
}

pub fn record(cut: Cut<'_>) -> Result<Record, String> {
    let git = datum::Git(cut.root);
    let current = git.at(cut.version, cut.head)?;
    let tree = super::version::Tree::open(cut.root, cut.head)?;
    if current.is_some() && tree.migrated(cut.version)? && tree.proved(cut.head)? {
        return Ok(Record {
            head: cut.head.to_string(),
            report: format!(
                "the {} datum is already recorded in Git metadata",
                cut.version
            ),
        });
    }
    let datum = tree.resolve(cut.version, cut.head)?;
    let refreshed = !tree.proved(cut.head)?;
    let head = tree.datum(cut.head, &datum)?;
    Ok(Record {
        head,
        report: format!(
            "recorded {} datum in Git metadata with {} answers{}",
            cut.version,
            datum.answers.len(),
            if refreshed {
                "; refreshed the release proof"
            } else {
                ""
            }
        ),
    })
}

pub(super) fn resolve(root: &Path, version: &str, head: &str) -> Result<Datum, String> {
    let git = datum::Git(root);
    if let Some(datum) = git.inherited(version, head)? {
        return Ok(datum);
    }
    if let Some(datum) = git.legacy(version, head)? {
        return Ok(datum);
    }
    Ok(Datum::new(version, dependency::answers(root)?))
}

impl Seat<'_> {
    pub fn carried(&self, commit: &str, version: &str) -> bool {
        let Ok(output) = std::process::Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(["diff-tree", "--no-commit-id", "--name-only", "-r", commit])
            .output()
        else {
            return false;
        };
        if !output.status.success() {
            return false;
        }
        let seat = format!("{}/{version}/", datum::SEAT);
        let touched = String::from_utf8_lossy(&output.stdout);
        if !touched.lines().all(|path| path.starts_with(&seat)) {
            return false;
        }
        datum::Git(self.0)
            .at(version, commit)
            .is_ok_and(|held| held.is_some())
            || (!touched.trim().is_empty()
                && datum::Git(self.0)
                    .legacy(version, commit)
                    .is_ok_and(|held| held.is_some()))
    }
}
