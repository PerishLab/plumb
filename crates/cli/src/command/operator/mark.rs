use super::course::Course;
use super::value;
use plumb::forgejo::Client;
use plumb::forgejo::git::fetch;
use std::path::Path;
use std::process::{Command, Output};

pub(super) fn stamp(raw: &str, dry: bool) -> Result<String, String> {
    let held = named(raw);
    let channel = super::super::release::channel(&held)?;
    let version = value::version(&held, &channel)?;
    let base = line(&version);
    let name = value::branch(&base);
    let root = plumb::forgejo::git::root()?;
    if channel == "stable" {
        let remote = plumb::forgejo::git::remote(&root, "")?;
        Client::new(remote)?
            .protected(&name, "frozen")
            .map_err(|error| format!("stable marker {held} requires a frozen {name}: {error}"))?;
    }
    let spec = crate::shape::release::Spec::controller(&root)?;
    let annotation = crate::command::release::annotation(&spec, &version)?;
    let seat = point(&root);
    let head = seat.prove(&name)?;
    seat.datum(&base, &head)?;
    let mut course = Course::new(dry);
    let said = course.step(
        format!("git tag -a {version} at {head} on {name}, then push"),
        || seat.stamp(&annotation, &version, &name, &head),
    )?;
    if course.dry() {
        return Ok(course.plan());
    }
    said.ok_or_else(|| "the stamp left no report".to_string())
}

fn named(raw: &str) -> String {
    if raw.starts_with('v') {
        raw.to_string()
    } else {
        format!("v{raw}")
    }
}

fn line(version: &str) -> String {
    version.split('-').next().unwrap_or(version).to_string()
}

pub struct Point<'a> {
    root: &'a Path,
}

pub fn point(root: &Path) -> Point<'_> {
    Point { root }
}

impl Point<'_> {
    fn datum(&self, version: &str, head: &str) -> Result<(), String> {
        let path = plumb::datum::leaf(version);
        let object = format!("{head}:{path}");
        let output = self.git(["show", &object])?;
        if !output.status.success() {
            return Err(format!(
                "release marker requires {path} at {head}; finish version prepare before stamping"
            ));
        }
        plumb::datum::decode(version, &output.stdout).map_err(|error| {
            format!("release marker requires a valid {path} at {head}: {error}")
        })?;
        Ok(())
    }

    pub fn head(&self, name: &str) -> Result<String, String> {
        fetch(self.root)?;
        text(
            "resolve release head",
            self.git(["rev-parse", &format!("origin/{name}")])?,
        )
    }

    pub fn prove(&self, name: &str) -> Result<String, String> {
        let head = self.head(name)?;
        let proof = plumb::guard::commit(self.root, &head)
            .map_err(|error| format!("release marker refuses an unproved tree: {error}"))?;
        proof
            .witness(self.root)
            .map_err(|error| format!("release marker refuses an unproved tree: {error}"))?;
        Ok(head)
    }

    pub fn stamp(
        &self,
        annotation: &str,
        version: &str,
        name: &str,
        head: &str,
    ) -> Result<String, String> {
        let standing = self.head(name)?;
        if standing != head {
            return Err(format!(
                "{name} moved from {head} to {standing} while its marker was being stamped"
            ));
        }
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
            self.git(["tag", "-a", version, head, "-m", annotation])?,
        )?;
        text(
            "publish the release point",
            self.git(["push", "origin", &reference(version)])?,
        )?;
        Ok(format!("stamped {version} at {head}"))
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
