use super::{Datum, carried, decode, leaf};
use std::path::Path;
use std::process::Command;

pub struct Git<'a>(pub &'a Path);

impl Git<'_> {
    pub fn current(&self, revision: &str) -> Result<Option<Datum>, String> {
        carried(&self.read(&["show", "-s", "--format=%B", revision])?)
    }

    pub fn at(&self, version: &str, revision: &str) -> Result<Option<Datum>, String> {
        let datum = self.current(revision)?;
        if datum.as_ref().is_some_and(|datum| datum.version != version) {
            return Ok(None);
        }
        Ok(datum)
    }

    pub fn inherited(&self, version: &str, revision: &str) -> Result<Option<Datum>, String> {
        let output = self.command(&[
            "log",
            "--first-parent",
            "--format=%B%x00",
            "--grep=^Plumb-Datum:",
            revision,
        ])?;
        if !output.status.success() {
            return Ok(None);
        }
        let body = String::from_utf8(output.stdout)
            .map_err(|error| format!("cannot read datum history: {error}"))?;
        for message in body.split('\0') {
            if let Some(datum) = carried(message)?
                && datum.version == version
            {
                return Ok(Some(datum));
            }
        }
        Ok(None)
    }

    pub fn legacy(&self, version: &str, revision: &str) -> Result<Option<Datum>, String> {
        let object = format!("{revision}:{}", leaf(version));
        let output = self.command(&["show", &object])?;
        if !output.status.success() {
            return Ok(None);
        }
        decode(version, &output.stdout).map(Some)
    }

    fn read(&self, args: &[&str]) -> Result<String, String> {
        let output = self.command(args)?;
        if !output.status.success() {
            return Err("cannot read datum commit".into());
        }
        String::from_utf8(output.stdout)
            .map_err(|error| format!("cannot read datum commit: {error}"))
    }

    fn command(&self, args: &[&str]) -> Result<std::process::Output, String> {
        Command::new("git")
            .arg("-C")
            .arg(self.0)
            .args(args)
            .output()
            .map_err(|error| format!("cannot read Git datum: {error}"))
    }
}
