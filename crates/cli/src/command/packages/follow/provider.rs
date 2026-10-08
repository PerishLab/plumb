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
        return Err(format!(
            "external command refused ({status}): {}",
            message.trim()
        ));
    }
    Ok(bytes)
}
