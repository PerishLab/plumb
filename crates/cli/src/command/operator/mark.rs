use super::course::Course;
use super::value;
use super::worktree::fetch;
use std::path::Path;
use std::process::{Command, Output};

pub(in crate::command) fn retract(raw: &str, dry: bool) -> Result<String, String> {
    let held = named(raw);
    let channel = super::super::release::channel(&held)?;
    let version = value::version(&held, &channel)?;
    let root = super::worktree::root()?;
    let authority = super::super::release::authority(&root)?;
    point(&root).retract(&authority, &channel, &version, dry)
}

fn named(raw: &str) -> String {
    if raw.starts_with('v') {
        raw.to_string()
    } else {
        format!("v{raw}")
    }
}

pub struct Point<'a> {
    root: &'a Path,
}

pub fn point(root: &Path) -> Point<'_> {
    Point { root }
}

impl Point<'_> {
    pub fn retract(
        &self,
        authority: &str,
        channel: &str,
        version: &str,
        dry: bool,
    ) -> Result<String, String> {
        let url = format!("{authority}/v1/releases/{channel}/{version}/seal.json");
        let mut course = Course::new(dry);
        if published(&url)? {
            return Err(format!(
                "{version} is published at {url}; retraction acts only on a declaration"
            ));
        }
        fetch(self.root)?;
        let point = reference(version);
        course.step(format!("git push origin --delete {point}"), || {
            text(
                "withdraw the release point",
                self.git(["push", "origin", "--delete", &point])?,
            )
        })?;
        if self.seen(version)?.is_some() {
            course.step(format!("git tag --delete {version}"), || {
                text(
                    "forget the release point",
                    self.git(["tag", "--delete", version])?,
                )
            })?;
        }
        if course.dry() {
            return Ok(course.plan());
        }
        Ok(format!("retracted {version}; it projected nothing"))
    }

    pub fn seen(&self, version: &str) -> Result<Option<String>, String> {
        let output = self.git(["rev-parse", &format!("{version}^{{commit}}")])?;
        if output.status.success() {
            Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
            ))
        } else {
            Ok(None)
        }
    }

    fn git<const N: usize>(&self, args: [&str; N]) -> Result<Output, String> {
        Command::new("git")
            .args(args)
            .current_dir(self.root)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))
    }
}

fn published(url: &str) -> Result<bool, String> {
    let response = plumb::vendor::send(url, "GET", None, None)
        .map_err(|error| format!("cannot read {url}: {error}"))?;
    match response.status {
        200 => Ok(true),
        404 => Ok(false),
        status => Err(format!(
            "release authority answered {status} for {url}; retraction refuses a reading it cannot trust"
        )),
    }
}

fn reference(version: &str) -> String {
    format!("refs/tags/{version}")
}

fn text(deed: &str, output: Output) -> Result<String, String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "cannot {deed}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
