use super::course::Course;
use super::owed::Seat;
use super::value;
use super::wharf::{git, reference, repository, text};
use crate::shape::release::Spec;
use plumb::land::rejoin::{Stable, tags};
use std::path::PathBuf;

const MAIN: &str = "main";

struct Line {
    root: PathBuf,
    remote: String,
    listing: String,
    spec: Spec,
}

impl Line {
    fn read(remote: &str) -> Result<Self, String> {
        let root = super::worktree::root()?;
        repository(&text(
            "read the remote",
            git(&root, &["remote", "get-url", remote])?,
        )?)?;
        let listing = text(
            "list the remote",
            git(&root, &["ls-remote", "--heads", "--tags", remote])?,
        )?;
        let spec = Spec::controller(&root)?;
        Ok(Self {
            root,
            remote: remote.to_string(),
            listing,
            spec,
        })
    }

    fn seat(&self) -> Seat<'_> {
        Seat {
            root: &self.root,
            remote: &self.remote,
            listing: &self.listing,
            spec: &self.spec,
        }
    }

    fn marked(&self, name: &str) -> Option<String> {
        tags(&self.listing)
            .into_iter()
            .find_map(|(held, commit)| (held == name).then(|| commit.to_string()))
    }

    fn origin(&self, from: &str, version: &str) -> Result<String, String> {
        if from == MAIN {
            return reference(&self.listing, "refs/heads/main")
                .ok_or_else(|| format!("{} has no main to open a line from", self.remote));
        }
        let floor = semver::Version::parse(from.trim_start_matches('v'))
            .ok()
            .filter(|held| held.pre.is_empty())
            .ok_or_else(|| format!("--from takes main or a stable marker, not {from}"))?;
        let ceiling = semver::Version::parse(version.trim_start_matches('v'))
            .map_err(|error| format!("invalid version {version}: {error}"))?;
        if floor >= ceiling {
            return Err(format!(
                "a line for {version} opens from below it, not from {from}"
            ));
        }
        self.marked(from)
            .ok_or_else(|| format!("{} holds no stable marker {from}", self.remote))
    }

    fn fetch(&self, reference: &str) -> Result<(), String> {
        text(
            "fetch the line's origin",
            git(&self.root, &["fetch", "--no-tags", &self.remote, reference])?,
        )
        .map(|_| ())
    }
}

pub(in crate::command) fn open(
    raw: &str,
    from: &str,
    remote: &str,
    dry: bool,
) -> Result<String, String> {
    let version = value::version(&named(raw), "stable")?;
    let branch = value::branch(&version);
    let line = Line::read(remote)?;
    if reference(&line.listing, &format!("refs/heads/{branch}")).is_some() {
        return Err(format!("{branch} is already open on {remote}"));
    }
    if line.marked(&version).is_some() {
        return Err(format!(
            "{version} already stands as stable; a released line never reopens"
        ));
    }
    line.seat().require(&version)?;
    let head = line.origin(from, &version)?;
    let source = if from == MAIN {
        "refs/heads/main".to_string()
    } else {
        format!("refs/tags/{from}")
    };
    let mut course = Course::new(dry);
    course.step(format!("git fetch {remote} {source}"), || {
        line.fetch(&source)
    })?;
    course.step(
        format!("git push {remote} {head}:refs/heads/{branch}"),
        || {
            text(
                "open the release line",
                git(
                    &line.root,
                    &["push", remote, &format!("{head}:refs/heads/{branch}")],
                )?,
            )
        },
    )?;
    if course.dry() {
        return Ok(course.plan());
    }
    Ok(format!("opened {branch} at {head} from {from}"))
}

pub(in crate::command) fn close(
    raw: &str,
    abandon: bool,
    remote: &str,
    dry: bool,
) -> Result<String, String> {
    let version = value::version(&named(raw), "stable")?;
    let branch = value::branch(&version);
    let line = Line::read(remote)?;
    let head = reference(&line.listing, &format!("refs/heads/{branch}"))
        .ok_or_else(|| format!("{remote} has no {branch}; nothing to close"))?;
    match (line.marked(&version), abandon) {
        (Some(commit), false) => released(&line, &version, &commit, &head)?,
        (Some(_), true) => {
            return Err(format!(
                "{version} stands as stable; close its line without --abandon"
            ));
        }
        (None, false) => {
            return Err(format!(
                "{version} has no stable marker; close an unreleased line with --abandon"
            ));
        }
        (None, true) => unreleased(&line, &version, &head, &branch)?,
    }
    let mut course = Course::new(dry);
    let lease = format!("--force-with-lease=refs/heads/{branch}:{head}");
    course.step(
        format!("git push {lease} {remote} :refs/heads/{branch}"),
        || {
            text(
                "close the release line",
                git(
                    &line.root,
                    &["push", &lease, remote, &format!(":refs/heads/{branch}")],
                )?,
            )
        },
    )?;
    if course.dry() {
        return Ok(course.plan());
    }
    Ok(format!(
        "closed {branch} at {head}; its markers keep its history"
    ))
}

pub(in crate::command) fn owed(version: Option<&str>, remote: &str) -> Result<String, String> {
    let line = Line::read(remote)?;
    let marker = version.map(named);
    line.seat().report(marker.as_deref())
}

fn released(line: &Line, version: &str, commit: &str, head: &str) -> Result<(), String> {
    if head != commit {
        return Err(format!(
            "release/{version} stands at {head}, past stable {version} at {commit}; land those commits into main or open another line before closing"
        ));
    }
    let stable = Stable {
        marker: version.to_string(),
        commit: commit.to_string(),
    };
    if !line.seat().rejoined(&stable)? {
        return Err(format!(
            "stable {version} is not yet in main; run plumb release rejoin before closing its line"
        ));
    }
    Ok(())
}

fn unreleased(line: &Line, version: &str, head: &str, branch: &str) -> Result<(), String> {
    let prefix = format!("{version}-");
    let recorded = tags(&line.listing)
        .into_iter()
        .any(|(name, commit)| name.starts_with(&prefix) && commit == head);
    if recorded {
        return Ok(());
    }
    Err(format!(
        "{branch} stands at {head}, which no marker of {version} records; abandoning it would lose those commits"
    ))
}

fn named(raw: &str) -> String {
    if raw.starts_with('v') {
        raw.to_string()
    } else {
        format!("v{raw}")
    }
}

#[cfg(test)]
mod proof;
