use super::repo::{Repository, success};
use super::{Refusal, refuse};
use serde::Serialize;
use std::path::{Path, PathBuf};

mod contain;
mod stable;
#[cfg(test)]
mod tests;

pub use contain::{Standing, contained, standing};
pub use stable::{Stable, latest, tags};

pub const SCHEMA: &str = "plumb.rejoin/v2";
const BASE: &str = "origin/main";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub root: PathBuf,
    pub marker: Option<String>,
    pub commit: Option<String>,
    pub main: Option<String>,
    pub state: &'static str,
    pub lacking: Vec<String>,
    pub conflicted: bool,
}

pub fn report(root: &Path) -> Result<Report, Refusal> {
    let repo = Repository::open(root)?;
    repo.fetch()?;
    let listing = repo.text(
        &["ls-remote", "--tags", "origin"],
        "remote",
        "cannot list the markers origin holds",
    )?;
    let mut report = Report {
        schema: SCHEMA,
        root: repo.root.clone(),
        marker: None,
        commit: None,
        main: None,
        state: "unmarked",
        lacking: Vec::new(),
        conflicted: false,
    };
    let Some(stable) = latest(tags(&listing)) else {
        return Ok(report);
    };
    let reference = format!("refs/tags/{}", stable.marker);
    let output = repo.git(&["fetch", "--no-tags", "origin", &reference])?;
    success(output, "remote", format!("cannot fetch {}", stable.marker))?;
    let main = repo.revision(BASE)?;
    let held = standing(&repo.root, &stable.commit, &main).map_err(|error| refuse("git", error))?;
    report.state = if held.home() { "home" } else { "owed" };
    report.conflicted = matches!(held, Standing::Conflicting(_));
    if let Standing::Lacking(paths) | Standing::Conflicting(paths) = held {
        report.lacking = paths;
    }
    report.marker = Some(stable.marker);
    report.commit = Some(stable.commit);
    report.main = Some(main);
    Ok(report)
}
