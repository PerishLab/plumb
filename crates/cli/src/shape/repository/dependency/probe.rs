use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const BYTES: u64 = 16 * 1024 * 1024;
const ERRORS: u64 = 65536;
const TIME: Duration = Duration::from_secs(30);

pub(super) struct Probe<'a> {
    pub root: &'a Path,
}

impl Probe<'_> {
    pub(super) fn read(&self) -> Result<Vec<u8>, String> {
        capture(
            crate::cargo::command()
                .args([
                    "metadata",
                    "--frozen",
                    "--all-features",
                    "--format-version",
                    "1",
                ])
                .current_dir(self.root),
        )
    }

    pub(super) fn source(&self) -> Result<String, String> {
        let bytes = capture(
            plumb::config::detached("git")
                .arg("-C")
                .arg(self.root)
                .args(["remote", "get-url", "origin"]),
        )?;
        let origin = std::str::from_utf8(&bytes)
            .map_err(|error| error.to_string())?
            .trim();
        let path = origin
            .strip_prefix("git@github.com:")
            .or_else(|| origin.strip_prefix("ssh://git@github.com/"))
            .or_else(|| origin.strip_prefix("https://github.com/"))
            .ok_or("topology origin is not a qualified GitHub repository")?;
        let path = path.strip_suffix(".git").unwrap_or(path);
        let parts = path.split('/').collect::<Vec<_>>();
        if parts.len() != 2
            || parts.iter().any(|part| {
                part.is_empty()
                    || !part
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            })
        {
            return Err("topology origin has no single owner/repository identity".into());
        }
        let owner = if parts[0].eq_ignore_ascii_case("PerishLab") {
            "PerishLab"
        } else {
            parts[0]
        };
        Ok(format!("{owner}/{}", parts[1]))
    }

    pub(super) fn tree(&self) -> Result<String, String> {
        let bytes = capture(
            plumb::config::detached("git")
                .arg("-C")
                .arg(self.root)
                .arg("write-tree"),
        )?;
        let tree = std::str::from_utf8(&bytes)
            .map_err(|error| error.to_string())?
            .trim();
        if tree.len() != 40 || !tree.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("topology has no exact Git tree".into());
        }
        Ok(tree.to_string())
    }

    pub(super) fn version(&self) -> Result<String, String> {
        let bytes = capture(
            crate::cargo::command()
                .arg("--version")
                .current_dir(self.root),
        )?;
        std::str::from_utf8(&bytes)
            .map(|text| text.trim().to_string())
            .map_err(|error| error.to_string())
    }
}

fn capture(command: &mut Command) -> Result<Vec<u8>, String> {
    let mut stdout = tempfile::tempfile().map_err(|error| error.to_string())?;
    let mut stderr = tempfile::tempfile().map_err(|error| error.to_string())?;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(stdout.try_clone().map_err(|error| error.to_string())?)
        .stderr(stderr.try_clone().map_err(|error| error.to_string())?)
        .spawn()
        .map_err(|error| format!("cannot start topology metadata: {error}"))?;
    let started = Instant::now();
    loop {
        let status = match child.try_wait() {
            Ok(status) => status,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
        };
        if let Some(status) = status {
            let bytes = bounded(&mut stdout, BYTES)?;
            let errors = bounded(&mut stderr, ERRORS)?;
            return if status.success() {
                Ok(bytes)
            } else {
                Err(format!(
                    "Cargo topology metadata refused: {}",
                    String::from_utf8_lossy(&errors).trim()
                ))
            };
        }
        let limit = match exceeded(started, &stdout, &stderr) {
            Ok(limit) => limit,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        if limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err("topology metadata exceeded its 30s or output budget".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn exceeded(started: Instant, stdout: &File, stderr: &File) -> Result<bool, String> {
    let stdout = stdout.metadata().map_err(|error| error.to_string())?.len();
    let stderr = stderr.metadata().map_err(|error| error.to_string())?.len();
    Ok(started.elapsed() >= TIME || stdout > BYTES || stderr > ERRORS)
}

fn bounded(file: &mut File, limit: u64) -> Result<Vec<u8>, String> {
    file.seek(SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("topology metadata exceeded its output budget".into());
    }
    Ok(bytes)
}
