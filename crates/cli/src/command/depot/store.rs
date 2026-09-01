use crate::command::release::storage::Authority;
use std::io::Write;
use std::process::{Command, Output};

mod generation;
mod readback;

pub struct Remote<'a> {
    held: &'a dyn Authority,
}

struct Rule<'a> {
    cache: &'a str,
    header: &'a str,
    value: &'a str,
}

impl<'a> Remote<'a> {
    pub fn new(held: &'a dyn Authority) -> Result<Self, String> {
        let secrets = [held.access(), held.secret(), held.bucket()];
        if secrets.iter().any(|value| value.is_empty()) {
            return Err("incomplete S3 depot authority".into());
        }
        if !held.endpoint().starts_with("https://") {
            return Err("incomplete S3 depot authority".into());
        }
        Ok(Self { held })
    }

    pub fn promote(&self, pointer: &plumb::depot::v2::Pointer) -> Result<String, String> {
        let latest = plumb::depot::v2::latest(
            &pointer.release.product,
            pointer.derivative,
            &pointer.release.channel,
        )?;
        self.advance(&latest, pointer, false)?;
        readback::projection(&pointer.source, &latest, pointer)?;
        Ok(format!(
            "advanced {} depot latest to {}",
            pointer.derivative.label(),
            pointer.release.version
        ))
    }

    fn create(&self, key: &str, bytes: &[u8]) -> Result<(), String> {
        let output = self.put(
            key,
            bytes,
            Rule {
                cache: "public, max-age=31536000, immutable",
                header: "--if-none-match",
                value: "*",
            },
        )?;
        if output.status.success() {
            return Ok(());
        }
        if !precondition(&output) {
            return Err(failure("create object", key, &output));
        }
        let standing = self
            .read(key)?
            .ok_or_else(|| format!("conditional create raced and {key} vanished"))?;
        if standing == bytes {
            Ok(())
        } else {
            Err(format!("immutable depot object drift: {key}"))
        }
    }

    fn advance(
        &self,
        key: &str,
        next: &plumb::depot::v2::Pointer,
        exact: bool,
    ) -> Result<(), String> {
        let text = next.encode()?;
        let bytes = text.as_bytes();
        let rule = Rule {
            cache: "public, max-age=60, must-revalidate",
            header: "--if-none-match",
            value: "*",
        };
        let Some(etag) = self.head(key)? else {
            let output = self.put(key, bytes, rule)?;
            return if output.status.success() {
                Ok(())
            } else {
                Err(failure("create pointer", key, &output))
            };
        };
        let standing = self
            .read(key)?
            .ok_or_else(|| format!("depot pointer vanished: {key}"))?;
        let standing = String::from_utf8(standing)
            .map_err(|error| format!("depot pointer at {key} is not UTF-8: {error}"))?;
        let current = plumb::depot::v2::Pointer::parse(&standing)?;
        if exact && current.release != next.release {
            return Err("exact depot pointer names another release".into());
        }
        if !current.advance(next)? {
            return Ok(());
        }
        let output = self.put(
            key,
            bytes,
            Rule {
                cache: rule.cache,
                header: "--if-match",
                value: &etag,
            },
        )?;
        if output.status.success() {
            Ok(())
        } else {
            Err(failure("move pointer", key, &output))
        }
    }

    fn put(&self, key: &str, bytes: &[u8], rule: Rule<'_>) -> Result<Output, String> {
        let mut body = tempfile::NamedTempFile::new()
            .map_err(|error| format!("cannot stage a depot object: {error}"))?;
        body.write_all(bytes)
            .map_err(|error| format!("cannot stage a depot object: {error}"))?;
        body.flush()
            .map_err(|error| format!("cannot stage a depot object: {error}"))?;
        self.command()
            .args(["put-object", "--bucket", self.held.bucket(), "--key", key])
            .arg("--body")
            .arg(body.path())
            .args([
                "--content-type",
                mime(key),
                "--cache-control",
                rule.cache,
                rule.header,
                rule.value,
                "--no-cli-pager",
            ])
            .output()
            .map_err(|error| format!("cannot run aws put-object: {error}"))
    }

    fn head(&self, key: &str) -> Result<Option<String>, String> {
        let output = self
            .command()
            .args([
                "head-object",
                "--bucket",
                self.held.bucket(),
                "--key",
                key,
                "--output",
                "json",
                "--no-cli-pager",
            ])
            .output()
            .map_err(|error| format!("cannot run aws head-object: {error}"))?;
        if !output.status.success() {
            return if absent(&output) {
                Ok(None)
            } else {
                Err(failure("head object", key, &output))
            };
        }
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
        value
            .get("ETag")
            .and_then(|held| held.as_str())
            .map(|held| Some(held.to_string()))
            .ok_or_else(|| format!("head-object returned no ETag for {key}"))
    }

    fn read(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        let body = tempfile::NamedTempFile::new()
            .map_err(|error| format!("cannot stage a depot read: {error}"))?;
        let output = self
            .command()
            .args(["get-object", "--bucket", self.held.bucket(), "--key", key])
            .arg(body.path())
            .arg("--no-cli-pager")
            .output()
            .map_err(|error| format!("cannot run aws get-object: {error}"))?;
        if output.status.success() {
            return std::fs::read(body.path())
                .map(Some)
                .map_err(|error| format!("cannot read {key}: {error}"));
        }
        if absent(&output) {
            Ok(None)
        } else {
            Err(failure("get object", key, &output))
        }
    }

    fn command(&self) -> Command {
        let mut held = Command::new("aws");
        held.env("AWS_ACCESS_KEY_ID", self.held.access())
            .env("AWS_SECRET_ACCESS_KEY", self.held.secret())
            .env("AWS_DEFAULT_REGION", "auto")
            .env("AWS_EC2_METADATA_DISABLED", "true")
            .arg("--endpoint-url")
            .arg(self.held.endpoint().trim_end_matches('/'))
            .arg("s3api");
        held
    }
}

fn mime(key: &str) -> &'static str {
    if key.ends_with(".json") {
        "application/json"
    } else {
        "text/plain; charset=utf-8"
    }
}

fn precondition(output: &Output) -> bool {
    let text = String::from_utf8_lossy(&output.stderr);
    text.contains("PreconditionFailed") || text.contains("412")
}

fn absent(output: &Output) -> bool {
    let text = String::from_utf8_lossy(&output.stderr);
    text.contains("NoSuchKey") || text.contains("Not Found") || text.contains("404")
}

fn failure(action: &str, key: &str, output: &Output) -> String {
    format!(
        "{action} {key}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
}
