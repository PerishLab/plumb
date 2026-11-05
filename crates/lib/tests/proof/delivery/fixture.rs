use plumb::delivery::{Patience, Settle, Squash};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::time::Duration;

pub(super) const HEAD: &str = "c0ffee0000000000000000000000000000000484";

pub(super) struct Fixture {
    pub checks: Value,
    pub workflows: Value,
    pub calls: RefCell<Vec<Vec<String>>>,
    pub fault: bool,
}

impl Fixture {
    pub fn new() -> Self {
        Self {
            checks: json!([{"check_runs": [Self::check(17), Self::check(18)]}]),
            workflows: json!({"17": Self::workflow(), "18": Self::workflow()}),
            calls: RefCell::default(),
            fault: false,
        }
    }

    pub fn check(id: u64) -> Value {
        json!({"name":"Guard", "app":{"slug":"github-actions"}, "head_sha":HEAD,
            "status":"completed", "conclusion":"success",
            "details_url":format!("https://github.com/Example/probe/actions/runs/{id}/job/1")})
    }

    fn workflow() -> Value {
        json!({"head_sha":HEAD, "path":".github/workflows/guard.yml", "event":"pull_request",
            "status":"completed", "conclusion":"success"})
    }

    pub fn settle(&self) -> Result<String, String> {
        let gh = |args: &[String]| self.command(args);
        let settle = Settle {
            repository: "Example/probe",
            noun: "fixture",
            patience: Patience {
                deadline: Duration::ZERO,
                pause: Duration::ZERO,
            },
            gh: &gh,
        };
        let guard = settle.guard(HEAD)?;
        settle.merge(
            23,
            &Squash {
                candidate: HEAD.into(),
                subject: "Qualify matching Guard runs".into(),
                body: "Refs #484".into(),
            },
        )?;
        Ok(guard)
    }

    fn command(&self, args: &[String]) -> Result<Vec<u8>, String> {
        self.calls.borrow_mut().push(args.to_vec());
        let value = match args
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice()
        {
            ["api", path, "--paginate", "--slurp"] => {
                assert_eq!(
                    path,
                    &format!("repos/Example/probe/commits/{HEAD}/check-runs?per_page=100")
                );
                self.checks.clone()
            }
            ["api", path] => {
                let id = path
                    .strip_prefix("repos/Example/probe/actions/runs/")
                    .unwrap();
                if self.fault && id == "18" {
                    return Err("provider read refused".into());
                }
                self.workflows[id].clone()
            }
            [
                "pr",
                "view",
                "23",
                "-R",
                "Example/probe",
                "--json",
                "headRefOid,mergeStateStatus",
            ] => {
                json!({"headRefOid":HEAD, "mergeStateStatus":"CLEAN"})
            }
            ["pr", "merge", ..] => Value::Null,
            _ => panic!("unexpected provider call: {args:?}"),
        };
        serde_json::to_vec(&value).map_err(|error| error.to_string())
    }

    pub fn merged(&self) -> bool {
        self.calls
            .borrow()
            .iter()
            .any(|call| call.starts_with(&["pr".into(), "merge".into()]))
    }
}
