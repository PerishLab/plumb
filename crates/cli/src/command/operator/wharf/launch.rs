use super::super::course::Course;
use super::{HUB, run, text};
use std::process::{Command, Stdio};

pub struct Launch<'a> {
    pub workflow: &'a str,
    pub fields: &'a [String],
    pub watch: bool,
}

pub fn launch(course: &mut Course, held: Launch<'_>) -> Result<Option<String>, String> {
    let mut args = vec!["workflow", "run", held.workflow, "-R", HUB];
    for field in held.fields {
        args.extend(["-f", field.as_str()]);
    }
    let launched = course.step(format!("gh {}", args.join(" ")), || {
        text("dispatch wharf", run(Command::new("gh").args(&args))?)
    })?;
    let Some(said) = launched else {
        return Ok(None);
    };
    let url = said
        .lines()
        .find(|line| line.contains("/actions/runs/"))
        .ok_or_else(|| format!("wharf dispatch named no run: {said}"))?
        .trim()
        .to_string();
    if !held.watch {
        return Ok(Some(url));
    }
    let id = url.rsplit('/').next().unwrap_or_default().to_string();
    let status = Command::new("gh")
        .args(["run", "watch", &id, "-R", HUB, "--exit-status"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| format!("cannot run gh: {error}"))?;
    if status.success() {
        Ok(Some(url))
    } else {
        Err(format!("{url} did not succeed"))
    }
}
