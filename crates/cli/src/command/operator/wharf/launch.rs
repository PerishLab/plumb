use super::super::course::Course;
use super::{HUB, run, text};
use std::process::Command;
use std::time::Duration;

const PATIENCE: u32 = 20;
const INTERVAL: Duration = Duration::from_secs(15);

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
    if held.watch && wait(&url)? != "success" {
        return Err(format!("{url} did not succeed"));
    }
    Ok(Some(url))
}

pub fn identity(url: &str) -> &str {
    url.rsplit('/').next().unwrap_or_default()
}

pub fn wait(url: &str) -> Result<String, String> {
    let mut failures = 0;
    let mut said = String::new();
    while failures < PATIENCE {
        let Some((status, conclusion)) = probe(identity(url)) else {
            failures += 1;
            std::thread::sleep(INTERVAL);
            continue;
        };
        failures = 0;
        announce(url, &status, &mut said);
        if status == "completed" {
            return Ok(conclusion);
        }
        std::thread::sleep(INTERVAL);
    }
    Err(format!("cannot read {url} after {PATIENCE} tries in a row"))
}

fn announce(url: &str, status: &str, said: &mut String) {
    if status != said {
        eprintln!("{url} {status}");
        *said = status.to_string();
    }
}

fn probe(id: &str) -> Option<(String, String)> {
    let output = Command::new("gh")
        .args(["run", "view", id, "-R", HUB, "--json", "status,conclusion"])
        .args(["--jq", r#".status + " " + .conclusion"#])
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let line = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let (status, conclusion) = line.split_once(' ').unwrap_or((line.as_str(), ""));
    Some((status.to_string(), conclusion.to_string()))
}
