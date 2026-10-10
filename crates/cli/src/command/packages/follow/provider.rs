use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(super) struct Provider<'a> {
    pub command: &'a Path,
    pub repository: &'a str,
}

impl Provider<'_> {
    pub fn api(&self, endpoint: &str, body: Option<&Value>) -> Result<Value, String> {
        self.request(endpoint, body, None)
    }

    pub fn patch(&self, endpoint: &str, body: &Value) -> Result<Value, String> {
        self.request(endpoint, Some(body), Some("PATCH"))
    }

    fn request(
        &self,
        endpoint: &str,
        body: Option<&Value>,
        method: Option<&str>,
    ) -> Result<Value, String> {
        let mut command = Command::new(self.command);
        command.args(["api", endpoint]);
        if let Some(method) = method {
            command.args(["--method", method]);
        }
        let mut payload = tempfile::NamedTempFile::new().map_err(|error| error.to_string())?;
        if let Some(body) = body {
            serde_json::to_writer(&mut payload, body).map_err(|error| error.to_string())?;
            payload.flush().map_err(|error| error.to_string())?;
            command.arg("--input").arg(payload.path());
        }
        let bytes = capture(command, Duration::from_secs(30))?;
        serde_json::from_slice(&bytes)
            .map_err(|error| format!("provider returned invalid JSON: {error}"))
    }

    pub fn command(&self, args: &[String]) -> Result<Vec<u8>, String> {
        let mut command = Command::new(self.command);
        command.args(args);
        capture(command, Duration::from_secs(30))
    }

    pub fn pages(&self, path: &str) -> Result<Vec<Value>, String> {
        let bytes = self.command(&[
            "api".into(),
            path.into(),
            "--paginate".into(),
            "--slurp".into(),
        ])?;
        let pages: Vec<Value> =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        Ok(pages
            .into_iter()
            .flat_map(|page| match page {
                Value::Array(items) => items,
                item => vec![item],
            })
            .collect())
    }

    pub fn issue(&self, number: u64) -> Result<Value, String> {
        self.api(&format!("repos/{}/issues/{number}", self.repository), None)
    }

    pub fn comment(&self, number: u64, body: &str) -> Result<(), String> {
        let endpoint = format!("repos/{}/issues/{number}/comments", self.repository);
        if self
            .pages(&format!("{endpoint}?per_page=100"))?
            .iter()
            .any(|comment| comment["body"] == body)
        {
            return Ok(());
        }
        self.api(&endpoint, Some(&json!({"body": body})))?;
        Ok(())
    }
}

pub(super) fn capture(mut command: Command, limit: Duration) -> Result<Vec<u8>, String> {
    let mut out = tempfile::tempfile().map_err(|error| error.to_string())?;
    let mut err = tempfile::tempfile().map_err(|error| error.to_string())?;
    command
        .stdin(Stdio::null())
        .stdout(out.try_clone().map_err(|error| error.to_string())?)
        .stderr(err.try_clone().map_err(|error| error.to_string())?);
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let started = Instant::now();
    let status = loop {
        if started.elapsed() > limit
            || out.metadata().map_err(|error| error.to_string())?.len() > 8 * 1024 * 1024
            || err.metadata().map_err(|error| error.to_string())?.len() > 65536
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(
                "external command exceeded its time or output bound; reread before retry".into(),
            );
        }
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    out.rewind().map_err(|error| error.to_string())?;
    err.rewind().map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    out.read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if !status.success() {
        let mut message = String::new();
        err.read_to_string(&mut message)
            .map_err(|error| error.to_string())?;
        if message.trim().is_empty() {
            message = String::from_utf8_lossy(&bytes).into_owned();
        }
        return Err(format!(
            "external command refused ({status}): {}",
            message.trim()
        ));
    }
    Ok(bytes)
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Intent {
    pub schema: String,
    pub repository: String,
    pub issue: u64,
    pub branch: String,
    pub previous: Option<String>,
    pub head: String,
    pub base: String,
    pub tree: String,
}

impl Intent {
    fn check(&self, provider: &Provider<'_>, issue: u64) -> Result<(), String> {
        let exact = |head: &str| {
            matches!(head.len(), 40 | 64) && head.bytes().all(|byte| byte.is_ascii_hexdigit())
        };
        let identity = (self.schema.as_str(), self.repository.as_str(), self.issue);
        if identity != ("plumb.auto-push/v1", provider.repository, issue)
            || self.branch != format!("auto/{issue}")
        {
            return Err("Auto publication intent identity disagrees".into());
        }
        if ![&self.head, &self.base, &self.tree]
            .into_iter()
            .all(|head| exact(head))
            || self.previous.as_deref().is_some_and(|head| !exact(head))
        {
            return Err("Auto publication intent has an invalid Git identity".into());
        }
        Ok(())
    }

    fn body(&self) -> Result<String, String> {
        Ok(format!(
            "Auto follow publication intent; provider readback decides execution.\n\n<!-- plumb.auto-push/v1\n{}\n-->",
            serde_json::to_string(self).map_err(|error| error.to_string())?
        ))
    }
}

impl Provider<'_> {
    pub(super) fn intent(&self, issue: u64, held: &Intent) -> Result<(), String> {
        held.check(self, issue)?;
        self.comment(issue, &held.body()?)?;
        if self.recorded(issue)?.as_ref() != Some(held) {
            return Err("Auto publication intent readback disagrees".into());
        }
        Ok(())
    }

    pub(super) fn recorded(&self, issue: u64) -> Result<Option<Intent>, String> {
        let owner = self.issue(issue)?["user"]["id"]
            .as_u64()
            .filter(|id| *id > 0)
            .ok_or("Auto issue has no operation writer identity")?;
        let comments = self.pages(&format!(
            "repos/{}/issues/{issue}/comments?per_page=100",
            self.repository
        ))?;
        let mut standing = None;
        for comment in comments {
            let body = comment["body"]
                .as_str()
                .ok_or("Auto comment is unreadable")?;
            if !body.contains("<!-- plumb.auto-push/") {
                continue;
            }
            if comment["user"]["id"].as_u64() != Some(owner) {
                return Err("Auto publication intent has another operation writer".into());
            }
            let value = body
                .split_once("<!-- plumb.auto-push/v1\n")
                .and_then(|(_, value)| value.strip_suffix("\n-->"))
                .ok_or("Auto publication intent has an unknown shape")?;
            let held: Intent = serde_json::from_str(value)
                .map_err(|error| format!("Auto publication intent is unreadable: {error}"))?;
            held.check(self, issue)?;
            standing = Some(held);
        }
        Ok(standing)
    }
}
