use super::api::Client;
use super::model::Outcome;
use serde_json::Value;

const JOBS: usize = 64;

impl Client {
    pub fn dispatch(
        &self,
        workflow: &str,
        reference: &str,
        inputs: Value,
    ) -> Result<Value, String> {
        self.call(&[
            "workflow",
            "dispatch",
            workflow,
            "--ref",
            reference,
            "--inputs",
            &inputs.to_string(),
        ])
    }

    pub fn run(&self, id: u64) -> Result<Value, String> {
        let value = self.call(&["run", "show", &id.to_string()])?;
        if value.is_object() {
            Ok(value)
        } else {
            Err("forgejo: workflow run response is not an object".into())
        }
    }

    pub fn outcome(&self, id: u64) -> Result<Outcome, String> {
        let run = self.run(id)?;
        let status = run
            .get("status")
            .and_then(Value::as_str)
            .ok_or_else(|| "Forgejo workflow run has no status".to_string())?;
        match status {
            "unknown" | "waiting" | "running" => Ok(Outcome::Waiting),
            "success" => {
                let number = number(&run)?;
                let tasks = self.tasks(number)?.unwrap_or_default();
                if flight(&tasks) {
                    Ok(Outcome::Waiting)
                } else {
                    Ok(Outcome::Success)
                }
            }
            "failure" | "cancelled" | "skipped" => {
                let number = number(&run)?;
                let tasks = self.tasks(number)?.unwrap_or_default();
                Ok(Outcome::Failed {
                    status: status.to_string(),
                    tasks: failed(&tasks),
                })
            }
            "blocked" => {
                let number = number(&run)?;
                blocked(self.tasks(number)?)
            }
            other => Err(format!("Forgejo workflow run has unknown status {other}")),
        }
    }

    fn tasks(&self, number: u64) -> Result<Option<Vec<Value>>, String> {
        let value = self.call(&["task", "list", &number.to_string()])?;
        value
            .as_array()
            .cloned()
            .map(Some)
            .ok_or_else(|| "Forgejo action task list is not an array".to_string())
    }

    pub fn log(&self, number: u64, job: usize, attempt: u32) -> Result<String, String> {
        let value = self.call(&[
            "job",
            "log",
            &number.to_string(),
            &job.to_string(),
            "--attempt",
            &attempt.to_string(),
        ])?;
        Ok(value
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| value.to_string()))
    }

    pub fn logs(&self, id: u64) -> Result<Vec<(usize, String)>, String> {
        let number = number(&self.run(id)?)?;
        let mut found = Vec::new();
        for job in 0..JOBS {
            match self.log(number, job, 1) {
                Ok(text) => found.push((job, text)),
                Err(_) if !found.is_empty() => break,
                Err(_) => continue,
            }
        }
        Ok(found)
    }

    pub fn link(&self, run: &Value) -> String {
        if let Some(url) = run.get("html_url").and_then(Value::as_str)
            && !url.trim().is_empty()
        {
            return url.trim().to_string();
        }
        run.get("run_number")
            .and_then(Value::as_u64)
            .map(|number| {
                format!(
                    "{}://{}/{}/{}/actions/runs/{number}",
                    self.remote.scheme, self.remote.host, self.remote.owner, self.remote.repo
                )
            })
            .unwrap_or_default()
    }
}

pub fn route(number: u64, job: usize, attempt: u32) -> String {
    format!("/actions/runs/{number}/jobs/{job}/attempt/{attempt}/logs")
}

fn number(run: &Value) -> Result<u64, String> {
    run.get("index_in_repo")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Forgejo workflow run has no repository run number".to_string())
}

fn failed(tasks: &[Value]) -> Vec<String> {
    let mut names = tasks
        .iter()
        .filter_map(|task| {
            let status = task.get("status").and_then(Value::as_str)?;
            matches!(status, "failure" | "cancelled" | "blocked").then(|| {
                task.get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(status)
                    .to_string()
            })
        })
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn flight(tasks: &[Value]) -> bool {
    tasks.iter().any(|task| {
        matches!(
            task.get("status").and_then(Value::as_str),
            Some("unknown" | "waiting" | "running")
        )
    })
}

fn blocked(tasks: Option<Vec<Value>>) -> Result<Outcome, String> {
    let Some(tasks) = tasks else {
        return Ok(Outcome::Failed {
            status: "blocked".into(),
            tasks: Vec::new(),
        });
    };
    let failures = failed(&tasks);
    if !failures.is_empty() {
        return Ok(Outcome::Failed {
            status: "blocked".into(),
            tasks: failures,
        });
    }
    let mut success = false;
    for task in &tasks {
        match task.get("status").and_then(Value::as_str) {
            Some("success") => success = true,
            Some("skipped") => {}
            Some("unknown" | "waiting" | "running") => return Ok(Outcome::Waiting),
            Some("failure" | "cancelled" | "blocked") => unreachable!(),
            Some(other) => return Err(format!("Forgejo action task has unknown status {other}")),
            None => return Err("Forgejo action task has no status".into()),
        }
    }
    if success {
        Ok(Outcome::Success)
    } else {
        Ok(Outcome::Failed {
            status: "blocked".into(),
            tasks: Vec::new(),
        })
    }
}
