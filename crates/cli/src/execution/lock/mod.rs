mod messages;
mod policy;

use messages::Messages;
use plumb::config::Execution;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;
use std::process::Stdio;

#[derive(Serialize)]
pub(crate) struct Report {
    schema: &'static str,
    tool: plumb::config::Tool,
    version: String,
    commit: Option<String>,
    #[serde(flatten)]
    scope: Scope,
    execution: String,
    tools: std::collections::BTreeMap<String, plumb::config::Tool>,
    configuration: String,
    compiler: String,
    clippy: String,
    argv: Vec<String>,
    guard: bool,
    limits: [&'static str; 6],
    messages: Messages,
}

#[derive(Serialize)]
pub(crate) struct Scope {
    #[serde(skip)]
    pub all: bool,
    pub source: String,
    pub tree: String,
    pub resolution: Option<plumb::packages::Resolution>,
}

impl Report {
    pub(crate) fn render(&self, json: bool) -> Result<String, String> {
        if json {
            return serde_json::to_string_pretty(self).map_err(|error| error.to_string());
        }
        let features = if self.scope.all {
            "all workspace"
        } else {
            "default"
        };
        Ok(format!(
            "lock resolution: {} compiler-confirmed native references; staged tree {}; host/{features} features, workspace/all-targets, no dependency linting; no Guard activation",
            self.messages.findings.len(),
            self.scope.tree
        ))
    }
}

pub(crate) fn resolve(root: &Path, execution: &Execution, scope: Scope) -> Result<Report, String> {
    super::inspect("cargo", root, &execution.environment)?;
    let config = policy::configuration(root)?;
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    std::fs::write(directory.path().join("clippy.toml"), &config)
        .map_err(|error| error.to_string())?;
    let compiler = version(execution, &["rustc", "-vV"])?;
    let clippy = version(execution, &["cargo", "clippy", "--version"])?;
    let hosts = compiler
        .lines()
        .filter_map(|line| line.strip_prefix("host: "))
        .collect::<Vec<_>>();
    let [host] = hosts.as_slice() else {
        return Err("rustc did not identify one compilation host".into());
    };
    if host.is_empty() {
        return Err("rustc compilation host is empty".into());
    }
    let mut argv = [
        "cargo",
        "clippy",
        "--offline",
        "--locked",
        "--workspace",
        "--all-targets",
        "--target",
        host,
        "--no-deps",
        "--message-format=json",
        "--",
        "--force-warn=clippy::disallowed_methods",
    ]
    .map(str::to_string)
    .to_vec();
    if scope.all {
        let at = argv
            .iter()
            .position(|arg| arg == "--")
            .expect("Cargo argument separator");
        argv.insert(at, "--all-features".into());
    }
    let limit = if scope.all {
        "staged local-locked Rust workspace, explicit host/all-targets with all workspace features"
    } else {
        "staged local-locked Rust workspace, explicit host/all-targets with default workspace features"
    };
    let mut output = tempfile::tempfile().map_err(|error| error.to_string())?;
    let mut command = execution.command("cargo")?;
    command
        .args(&argv[1..])
        .env("CLIPPY_CONF_DIR", directory.path())
        .stdin(Stdio::null())
        .stdout(output.try_clone().map_err(|error| error.to_string())?)
        .stderr(Stdio::inherit());
    let cache = super::cargo::cache(Some(execution))?;
    if let Some(cache) = &cache {
        cache.apply(&mut command);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot run Cargo lock resolution: {error}"))?;
    let status = super::signal::wait(&mut child, &argv)?;
    drop(cache);
    execution.verify()?;
    if output.metadata().map_err(|error| error.to_string())?.len() > 67108864 {
        return Err("Cargo lock resolution exceeds 64 MiB of machine output".into());
    }
    output
        .seek(SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    let messages = messages::read(BufReader::new(output))?;
    if !status.success() {
        return Err(format!("Cargo lock resolution failed with {status}"));
    }
    Ok(Report {
        schema: "plumb.lock-resolution/v1",
        tool: plumb::config::Tool::at(std::env::current_exe().map_err(|error| error.to_string())?)?,
        version: plumb::version!("PLUMB").to_string(),
        commit: plumb::commit!("PLUMB").map(str::to_string),
        scope,
        execution: format!("{:x}", Sha256::digest(execution.evidence()?)),
        tools: execution.tools()?,
        configuration: format!("{:x}", Sha256::digest(config.as_bytes())),
        compiler,
        clippy,
        argv,
        guard: false,
        limits: [
            limit,
            "compiler artifacts include dependencies; --no-deps limits linting to selected workspace units",
            "only five std::fs::File locking methods; no non-Rust, third-party locking API or unselected cfg coverage",
            "references and complete expansion spans are evidence, not role, lifetime, disposition or admission judgments",
            "Cargo diagnostics can repeat across lib/test units; artifact metadata is not an individual diagnostic cfg proof",
            "--all-features selects one workspace feature combination, not every combination or transitive optional branch",
        ],
        messages,
    })
}

fn version(execution: &Execution, argv: &[&str]) -> Result<String, String> {
    let output = execution.output(
        &argv
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>(),
    )?;
    if !output.status.success() {
        return Err(format!("{} failed with {}", argv.join(" "), output.status));
    }
    String::from_utf8(output.stdout).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
