mod intent;
mod remote;

pub(super) use intent::Intent;

use crate::command::diagnostic;
use runseal::github::{Client, records::Author};
use serde::Serialize;
use serde_json::Value;
#[cfg(all(test, unix))]
use std::io::Write;
use std::io::{Read, Seek};
#[cfg(all(test, unix))]
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(super) struct Provider<'a> {
    pub source: Source,
    #[cfg(all(test, unix))]
    pub command: &'a Path,
    pub repository: &'a str,
    pub patience: Patience,
    pub sources: plumb::packages::Sources,
}

pub(super) enum Source {
    Native(Box<Client>),
    #[cfg(all(test, unix))]
    Fixture,
}

pub(super) use plumb::delivery::Patience;

impl<'a> Provider<'a> {
    pub(super) fn native(repository: &'a str) -> Result<Self, String> {
        Ok(Self {
            source: Source::Native(Box::new(crate::command::github()?)),
            #[cfg(all(test, unix))]
            command: Path::new(""),
            repository,
            patience: Patience::MERGE,
            sources: Default::default(),
        })
    }
}

impl Provider<'_> {
    #[cfg(all(test, unix))]
    pub fn api(&self, endpoint: &str, body: Option<&Value>) -> Result<Value, String> {
        self.request(endpoint, body, None)
    }

    #[cfg(all(test, unix))]
    pub fn patch(&self, endpoint: &str, body: &Value) -> Result<Value, String> {
        self.request(endpoint, Some(body), Some("PATCH"))
    }

    #[cfg(all(test, unix))]
    fn request(
        &self,
        endpoint: &str,
        body: Option<&Value>,
        method: Option<&str>,
    ) -> Result<Value, String> {
        let mut command = self.process();
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

    #[cfg(all(test, unix))]
    pub fn command(&self, args: &[String]) -> Result<Vec<u8>, String> {
        let mut command = self.process();
        command.args(args);
        capture(command, Duration::from_secs(30))
    }

    #[cfg(all(test, unix))]
    fn process(&self) -> Command {
        let mut command = Command::new(self.command);
        command.env("GH_HOST", "github.com");
        command
    }

    #[cfg(all(test, unix))]
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

    #[cfg(all(test, unix))]
    pub fn settle<T>(&self, then: impl FnOnce(&plumb::delivery::Settle<'_>) -> T) -> T {
        let gh = |args: &[String]| self.command(args);
        then(&plumb::delivery::Settle {
            repository: self.repository,
            noun: "Auto",
            patience: self.patience,
            gh: &gh,
        })
    }

    pub fn issue(&self, number: u64) -> Result<Value, String> {
        match &self.source {
            Source::Native(client) => encode(
                client
                    .ticket(&self.target(number)?)
                    .map_err(diagnostic)?
                    .value,
            ),
            #[cfg(all(test, unix))]
            Source::Fixture => {
                self.api(&format!("repos/{}/issues/{number}", self.repository), None)
            }
        }
    }

    pub(super) fn coordinate(&self) -> Result<(&str, &str), String> {
        self.repository
            .split_once('/')
            .ok_or_else(|| "invalid GitHub repository coordinate".into())
    }

    pub(super) fn viewer(&self) -> Result<Author, String> {
        match &self.source {
            Source::Native(client) => {
                let actor = client.viewer().map_err(diagnostic)?.value;
                Ok(Author {
                    id: actor.number,
                    login: actor.login,
                })
            }
            #[cfg(all(test, unix))]
            Source::Fixture => {
                let args = [
                    "api",
                    "graphql",
                    "-f",
                    "query=query{viewer{databaseId login}}",
                ];
                let reply = self.command(&args.map(str::to_owned))?;
                let value: Value = serde_json::from_slice(&reply).map_err(|err| err.to_string())?;
                let actor = &value["data"]["viewer"];
                serde_json::from_value(
                    serde_json::json!({"id":actor["databaseId"],"login":actor["login"]}),
                )
                .map_err(|_| "provider identity is unreadable".into())
            }
        }
    }

    pub fn comment(&self, number: u64, body: &str) -> Result<(), String> {
        let issue = self.issue(number)?;
        let owner = super::issue::author(&issue)?;
        if self
            .discussion(number)?
            .iter()
            .any(|comment| comment["body"] == body && comment["user"]["id"].as_u64() == Some(owner))
        {
            return Ok(());
        }
        match &self.source {
            Source::Native(client) => client
                .comment(&super::issue::text(&issue, "node_id")?, body)
                .map(|_| ())
                .map_err(diagnostic),
            #[cfg(all(test, unix))]
            Source::Fixture => self
                .api(
                    &format!("repos/{}/issues/{number}/comments", self.repository),
                    Some(&serde_json::json!({"body": body})),
                )
                .map(|_| ()),
        }
    }
}

pub(super) fn encode(value: impl Serialize) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|error| error.to_string())
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
