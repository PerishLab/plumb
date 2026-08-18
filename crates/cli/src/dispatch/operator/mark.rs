use super::course::Course;
use plumb::forgejo::git::fetch;
use std::path::Path;
use std::process::{Command, Output};

pub struct Point<'a> {
    root: &'a Path,
}

pub fn point(root: &Path) -> Point<'_> {
    Point { root }
}

impl Point<'_> {
    pub fn stamp(&self, product: &str, version: &str, name: &str) -> Result<String, String> {
        fetch(self.root)?;
        let head = text(
            "resolve release head",
            self.git(["rev-parse", &format!("origin/{name}")])?,
        )?;
        if let Some(seen) = self.seen(version)? {
            return if seen == head {
                Ok(format!("{version} already stands at {head}"))
            } else {
                Err(format!(
                    "{version} stands at {seen}, not {name} at {head}; a stamped point never moves"
                ))
            };
        }
        text(
            "stamp the release point",
            self.git([
                "tag",
                "-a",
                version,
                &head,
                "-m",
                &format!("{product} {version}"),
            ])?,
        )?;
        text(
            "publish the release point",
            self.git(["push", "origin", &reference(version)])?,
        )?;
        Ok(format!("stamped {version} at {head}"))
    }

    pub fn retract(&self, authority: &str, version: &str, dry: bool) -> Result<String, String> {
        let url = format!("{authority}/v1/releases/stable/{version}/seal.json");
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

    fn seen(&self, version: &str) -> Result<Option<String>, String> {
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
