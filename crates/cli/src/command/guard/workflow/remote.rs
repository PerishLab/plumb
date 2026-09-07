use plumb::rig::Authority;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::process::{Command, Output};

use super::reuse::{Inventory, Keys, Record};

pub const SEAT: &str = "v1";

pub struct Remote<'a>(&'a Authority);

impl<'a> Remote<'a> {
    pub fn new(held: &'a Authority) -> Self {
        Self(held)
    }
}

impl Remote<'_> {
    fn ready(&self) -> Result<(), String> {
        let held = [
            self.0.access.as_str(),
            self.0.secret.as_str(),
            self.0.bucket.as_str(),
        ];
        if held.iter().any(|value| value.is_empty()) || !self.0.endpoint.starts_with("https://") {
            return Err("incomplete S3 lock authority".to_string());
        }
        Ok(())
    }

    pub fn read(&self, key: &str) -> Result<Option<String>, String> {
        self.ready()?;
        let seat = key.to_string();
        let path = PathBuf::from(format!(".plumb-lock-{}", std::process::id()));
        let output = self
            .command()
            .args(["get-object", "--bucket", &self.0.bucket, "--key", &seat])
            .arg(&path)
            .arg("--no-cli-pager")
            .output()
            .map_err(|error| format!("cannot run aws get-object: {error}"))?;
        if absent(&output) {
            return Ok(None);
        }
        if !output.status.success() {
            let _ = std::fs::remove_file(&path);
            return Err(failure("read lock", &seat, &output));
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read the drawn lock: {error}"))?;
        let _ = std::fs::remove_file(&path);
        Ok(Some(text.trim().to_string()))
    }

    pub fn write(&self, key: &str, digest: &str) -> Result<(), String> {
        self.ready()?;
        let seat = key.to_string();
        let path = PathBuf::from(format!(".plumb-lock-{}.put", std::process::id()));
        std::fs::write(&path, format!("{digest}\n"))
            .map_err(|error| format!("cannot stage the lock: {error}"))?;
        let etag = self.etag(&seat)?;
        let (header, value) = match &etag {
            Some(etag) => ("--if-match", etag.as_str()),
            None => ("--if-none-match", "*"),
        };
        let output = self
            .command()
            .args(["put-object", "--bucket", &self.0.bucket, "--key", &seat])
            .arg("--body")
            .arg(&path)
            .args([
                "--content-type",
                "text/plain",
                "--cache-control",
                "no-store",
                header,
                value,
                "--no-cli-pager",
            ])
            .output()
            .map_err(|error| format!("cannot run aws put-object: {error}"))?;
        let _ = std::fs::remove_file(&path);
        if output.status.success() {
            return Ok(());
        }
        Err(failure("write lock", &seat, &output))
    }

    fn etag(&self, seat: &str) -> Result<Option<String>, String> {
        let output = self
            .command()
            .args([
                "head-object",
                "--bucket",
                &self.0.bucket,
                "--key",
                seat,
                "--output",
                "json",
                "--no-cli-pager",
            ])
            .output()
            .map_err(|error| format!("cannot run aws head-object: {error}"))?;
        if absent(&output) {
            return Ok(None);
        }
        if !output.status.success() {
            return Err(failure("head lock", seat, &output));
        }
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
        Ok(value
            .get("ETag")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string))
    }

    fn command(&self) -> Command {
        let mut held = Command::new("aws");
        held.env("AWS_ACCESS_KEY_ID", &self.0.access)
            .env("AWS_SECRET_ACCESS_KEY", &self.0.secret)
            .env("AWS_DEFAULT_REGION", "auto")
            .env("AWS_EC2_METADATA_DISABLED", "true")
            .arg("--endpoint-url")
            .arg(self.0.endpoint.trim_end_matches('/'))
            .arg("s3api");
        held
    }
}

fn absent(output: &Output) -> bool {
    let text = String::from_utf8_lossy(&output.stderr);
    text.contains("NoSuchKey") || text.contains("Not Found") || text.contains("404")
}

fn failure(action: &str, seat: &str, output: &Output) -> String {
    format!(
        "{action} {seat}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

impl Inventory {
    pub fn at(mut self, url: Option<&str>) -> Result<Self, String> {
        let Some(url) = url else {
            return Ok(self);
        };
        let suffix = "/inventory.json";
        self.base = Some(
            url.strip_suffix(suffix)
                .ok_or_else(|| format!("workflow inventory URL must end in {suffix}"))?
                .to_string(),
        );
        Ok(self)
    }

    pub(super) fn enrich(&self, action: &str, keys: &Keys) -> Result<Self, String> {
        let mut held = Self {
            schema: self.schema.clone(),
            records: self.records.clone(),
            base: None,
            direct: Vec::new(),
        };
        let Some(base) = &self.base else {
            return Ok(held);
        };
        for route in lookup(action, keys) {
            let body = match plumb::bucket::fetch(&format!("{base}/{route}")) {
                Ok(Some(body)) => body,
                Ok(None) | Err(_) => continue,
            };
            let record = Record::decode(&body)?;
            if !held.direct.contains(&record) {
                held.direct.push(record);
            }
        }
        Ok(held)
    }
}

impl Record {
    pub(super) fn routes(&self) -> Vec<(String, Self)> {
        match self.source.kind.as_str() {
            "workload" => {
                let mut content = self.clone();
                content.proof = None;
                vec![
                    (
                        route(
                            "workload-proof",
                            &[
                                &self.action,
                                &self.workload,
                                self.proof.as_deref().unwrap_or_default(),
                            ],
                        ),
                        self.clone(),
                    ),
                    (route("workload", &[&self.action, &self.workload]), content),
                ]
            }
            "url" => vec![(
                route(
                    "publication",
                    &[
                        &self.action,
                        &self.workload,
                        self.publication.as_deref().unwrap_or_default(),
                    ],
                ),
                self.clone(),
            )],
            _ => Vec::new(),
        }
    }
}

fn lookup(action: &str, keys: &Keys) -> Vec<String> {
    let mut found = Vec::new();
    if let Some(publication) = &keys.publication {
        found.push(route("publication", &[action, &keys.workload, publication]));
    }
    found.push(route(
        "workload-proof",
        &[action, &keys.workload, &keys.proof],
    ));
    found.push(route("workload", &[action, &keys.workload]));
    found
}

fn route(kind: &str, values: &[&str]) -> String {
    let mut sponge = Sha256::new();
    for value in values {
        sponge.update(value.as_bytes());
        sponge.update([0]);
    }
    format!("records/{kind}/{:x}.json", sponge.finalize())
}
