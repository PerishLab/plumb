use super::Squash;
use serde_json::Value;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Patience {
    pub deadline: Duration,
    pub pause: Duration,
}

impl Patience {
    pub const MERGE: Self = Self {
        deadline: Duration::from_secs(300),
        pause: Duration::from_secs(10),
    };
}

pub type Runner<'a> = dyn Fn(&[String]) -> Result<Vec<u8>, String> + 'a;

pub struct Settle<'a> {
    pub repository: &'a str,
    pub noun: &'a str,
    pub patience: Patience,
    pub gh: &'a Runner<'a>,
}

impl Settle<'_> {
    pub fn guard(&self, head: &str) -> Result<String, String> {
        let pages = self.json(&[
            "api",
            &format!(
                "repos/{}/commits/{head}/check-runs?per_page=100",
                self.repository
            ),
            "--paginate",
            "--slurp",
        ])?;
        let pages = pages.as_array().ok_or("check-run response is unreadable")?;
        let mut checks = Vec::new();
        for page in pages.iter().flat_map(|page| match page {
            Value::Array(items) => items.iter().collect(),
            item => vec![item],
        }) {
            checks.extend_from_slice(
                page["check_runs"]
                    .as_array()
                    .ok_or("check-run response is unreadable")?,
            );
        }
        Guard {
            head,
            noun: self.noun,
        }
        .observe(&checks, |id| {
            self.json(&[
                "api",
                &format!("repos/{}/actions/runs/{id}", self.repository),
            ])
        })
    }

    pub fn wait(&self, head: &str) -> Result<String, String> {
        Guard::wait(self.patience.pause, || self.guard(head))
    }

    pub fn merge(&self, pull: u64, squash: &Squash) -> Result<(), String> {
        let Patience { deadline, pause } = self.patience;
        let deadline = Instant::now() + deadline;
        loop {
            self.ready(pull, &squash.candidate, deadline)?;
            match (self.gh)(&squash.arguments(self.repository, pull)) {
                Ok(_) => return Ok(()),
                Err(error) if refused(&error) && Instant::now() + pause < deadline => {
                    std::thread::sleep(pause)
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn ready(&self, pull: u64, candidate: &str, deadline: Instant) -> Result<(), String> {
        let noun = self.noun;
        let pause = self.patience.pause;
        loop {
            let bytes = (self.gh)(
                &[
                    "pr",
                    "view",
                    &pull.to_string(),
                    "-R",
                    self.repository,
                    "--json",
                    "headRefOid,mergeStateStatus",
                ]
                .map(str::to_string),
            )?;
            let view: Value = serde_json::from_slice(&bytes)
                .map_err(|error| format!("{noun} pull readiness is unreadable: {error}"))?;
            let head = text(&view, "headRefOid")?;
            let status = text(&view, "mergeStateStatus")?;
            if head != candidate {
                return Err(format!(
                    "{noun} pull head {head} is not the exact candidate {candidate}; merge refused"
                ));
            }
            match status.as_str() {
                "CLEAN" => return Ok(()),
                "BLOCKED" | "UNKNOWN" if Instant::now() + pause < deadline => {
                    std::thread::sleep(pause)
                }
                "BLOCKED" | "UNKNOWN" => {
                    return Err(format!(
                        "{noun} pull did not become mergeable within the bounded wait; last mergeStateStatus {status}"
                    ));
                }
                _ => {
                    return Err(format!(
                        "{noun} pull is not mergeable: mergeStateStatus {status}; merge refused"
                    ));
                }
            }
        }
    }

    fn json(&self, args: &[&str]) -> Result<Value, String> {
        let args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        let bytes = (self.gh)(&args)?;
        serde_json::from_slice(&bytes)
            .map_err(|error| format!("provider returned invalid JSON: {error}"))
    }
}

pub struct Guard<'a> {
    pub head: &'a str,
    pub noun: &'a str,
}

impl Guard<'_> {
    pub fn observe(
        &self,
        checks: &[Value],
        workflow: impl Fn(&str) -> Result<Value, String>,
    ) -> Result<String, String> {
        let matches = checks
            .iter()
            .filter(|run| {
                run["name"] == "Guard"
                    && run["app"]["slug"] == "github-actions"
                    && run["head_sha"] == self.head
            })
            .collect::<Vec<_>>();
        if matches.is_empty() {
            return Err(format!(
                "organization Guard is pending; no run on the exact {} head",
                self.noun
            ));
        }
        let url = self.workflow(matches[0], &workflow)?;
        for run in &matches[1..] {
            self.workflow(run, &workflow)?;
        }
        Ok(url)
    }

    fn workflow(
        &self,
        run: &Value,
        workflow: &impl Fn(&str) -> Result<Value, String>,
    ) -> Result<String, String> {
        if run["status"] != "completed" {
            return Err("organization Guard is pending; rerun after its check completes".into());
        }
        let conclusion = run["conclusion"]
            .as_str()
            .filter(|text| !text.is_empty())
            .ok_or("completed organization Guard has no readable conclusion")?;
        let url = text(run, "details_url")?;
        let id = url
            .split("/actions/runs/")
            .nth(1)
            .and_then(|tail| tail.split('/').next())
            .filter(|id| id.parse::<u64>().is_ok())
            .ok_or("Guard check has no workflow run identity")?;
        let workflow = workflow(id)?;
        if workflow["head_sha"] != self.head
            || workflow["path"] != ".github/workflows/guard.yml"
            || !matches!(
                workflow["event"].as_str(),
                Some("pull_request" | "merge_group")
            )
        {
            return Err("Guard check is not the organization workflow on this exact head".into());
        }
        if conclusion != "success" {
            return Err(format!("organization Guard failed: {}", run["conclusion"]));
        }
        if workflow["status"] != "completed" {
            return Err(format!(
                "organization Guard is pending; workflow run {id} is {} after its Guard check completed",
                workflow["status"]
            ));
        }
        if workflow["conclusion"] != "success" {
            return Err(format!(
                "organization Guard failed: workflow run {id} concluded {}",
                workflow["conclusion"]
            ));
        }
        Ok(url)
    }

    pub fn wait(
        pause: Duration,
        probe: impl Fn() -> Result<String, String>,
    ) -> Result<String, String> {
        for _ in 0..90 {
            match probe() {
                Ok(run) => return Ok(run),
                Err(error) if error.starts_with("organization Guard is pending;") => {
                    std::thread::sleep(pause)
                }
                Err(error) => return Err(error),
            }
        }
        Err("organization Guard did not settle within the bounded wait".into())
    }
}

fn refused(error: &str) -> bool {
    [
        "base branch policy prohibits the merge",
        "not mergeable",
        "is expected",
        "mergeability",
    ]
    .iter()
    .any(|held| error.contains(held))
}

fn text(value: &Value, key: &str) -> Result<String, String> {
    value[key]
        .as_str()
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("provider field {key} is missing"))
}
