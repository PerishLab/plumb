use serde_json::Value;
use std::process::Command;

pub struct Store {
    access: String,
    secret: String,
    bucket: String,
    endpoint: String,
}

pub struct Held {
    pub keys: Vec<String>,
    pub bytes: u64,
}

impl Store {
    pub fn new(access: &str, secret: &str, seat: &Seat) -> Self {
        Self {
            access: access.to_string(),
            secret: secret.to_string(),
            bucket: seat.bucket.clone(),
            endpoint: seat.endpoint.clone(),
        }
    }

    pub fn inventory(&self) -> Result<Held, String> {
        let mut keys = Vec::new();
        let mut bytes = 0;
        let mut token = String::new();
        loop {
            let value = self.list(&token)?;
            for entry in value
                .get("Contents")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(key) = entry.get("Key").and_then(Value::as_str) {
                    keys.push(key.to_string());
                }
                bytes += entry
                    .get("Size")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
            }
            match value.get("NextContinuationToken").and_then(Value::as_str) {
                Some(next) => token = next.to_string(),
                None => return Ok(Held { keys, bytes }),
            }
        }
    }

    pub fn empty(&self, keys: &[String]) -> Result<(), String> {
        for key in keys {
            let output = self
                .command()
                .args(["delete-object", "--bucket", &self.bucket, "--key", key])
                .output()
                .map_err(|error| format!("cannot run aws delete-object: {error}"))?;
            if !output.status.success() {
                return Err(format!(
                    "deleting {key} failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
        }
        Ok(())
    }

    fn list(&self, token: &str) -> Result<Value, String> {
        let mut held = self.command();
        held.args([
            "list-objects-v2",
            "--bucket",
            &self.bucket,
            "--output",
            "json",
        ]);
        if !token.is_empty() {
            held.args(["--continuation-token", token]);
        }
        let output = held
            .output()
            .map_err(|error| format!("cannot run aws list-objects-v2: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "listing {} failed: {}",
                self.bucket,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("cannot parse object listing: {error}"))
    }

    fn command(&self) -> Command {
        let mut held = Command::new("aws");
        held.env("AWS_ACCESS_KEY_ID", &self.access)
            .env("AWS_SECRET_ACCESS_KEY", &self.secret)
            .env("AWS_DEFAULT_REGION", "auto")
            .env("AWS_EC2_METADATA_DISABLED", "true")
            .arg("--endpoint-url")
            .arg(self.endpoint.trim_end_matches('/'))
            .arg("s3api");
        held
    }
}

pub struct Seat {
    pub bucket: String,
    pub endpoint: String,
}

pub fn digest(value: &str) -> String {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    hasher.update(value.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
