use super::record::{Capsule, Local, Pointer};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub trait Authority {
    fn access(&self) -> &str;
    fn secret(&self) -> &str;
    fn bucket(&self) -> &str;
    fn endpoint(&self) -> &str;
}

pub fn publish(path: &Path, authority: &impl Authority) -> Result<String, String> {
    let (capsule, root) = Capsule::read(path)?;
    let remote = Remote::new(authority)?;
    for object in &capsule.objects {
        remote.create(object, &root, true)?;
    }
    remote.create(&capsule.seal, &root, true)?;
    if let Err(error) = super::verify::published(&capsule) {
        remote.exact(
            &capsule.seal,
            "release seal vanished before public projection",
        )?;
        return Err(format!(
            "{error}; release authority endpoint fingerprint {}",
            remote.fingerprint()
        ));
    }
    let channel = capsule.channel;
    Ok(format!("published exact {channel} {}", capsule.version))
}

pub(super) struct Remote<'a> {
    held: &'a dyn Authority,
}

struct Rule<'a> {
    cache: &'a str,
    header: &'a str,
    value: &'a str,
}

impl<'a> Remote<'a> {
    pub(super) fn new(held: &'a dyn Authority) -> Result<Self, String> {
        let secrets = [held.access(), held.secret(), held.bucket()];
        if secrets.iter().any(|value| value.is_empty()) {
            return Err("incomplete S3 release authority".into());
        }
        if !held.endpoint().starts_with("https://") {
            return Err("incomplete S3 release authority".into());
        }
        Ok(Self { held })
    }

    fn create(&self, object: &Local, root: &Path, immutable: bool) -> Result<(), String> {
        let source = root.join(&object.source);
        let cache = if immutable {
            "public, max-age=31536000, immutable"
        } else {
            "public, max-age=60, must-revalidate"
        };
        let output = self.put(
            object,
            &source,
            Rule {
                cache,
                header: "--if-none-match",
                value: "*",
            },
        )?;
        if output.status.success() {
            return self.exact(
                object,
                "created object is not readable from release authority",
            );
        }
        if !precondition(&output) {
            return Err(failure("create object", &object.key, &output));
        }
        self.exact(object, "conditional create raced and object vanished")
    }

    fn exact(&self, object: &Local, absent: &str) -> Result<(), String> {
        let remote = self.get(&object.key)?;
        let Some(path) = remote else {
            return Err(format!("{absent}: {}", object.key));
        };
        let result = super::record::digest(&path).and_then(|(digest, size)| {
            if digest == object.remote.sha256 && size == object.remote.size {
                Ok(())
            } else {
                Err(format!("immutable object drift: {}", object.key))
            }
        });
        let _ = std::fs::remove_file(path);
        result
    }

    pub(super) fn shift(&self, object: &Local, root: &Path) -> Result<(), String> {
        let source = root.join(&object.source);
        match self.head(&object.key)? {
            None => self.create(object, root, false),
            Some(_) if self.same(object)? => Ok(()),
            Some(etag) => {
                let output = self.put(
                    object,
                    &source,
                    Rule {
                        cache: "public, max-age=60, must-revalidate",
                        header: "--if-match",
                        value: &etag,
                    },
                )?;
                if !output.status.success() {
                    return Err(failure("move object", &object.key, &output));
                }
                Ok(())
            }
        }
    }

    pub(super) fn activate(&self, object: &Local, root: &Path) -> Result<(), String> {
        let source = root.join(&object.source);
        let prior = self.head(&object.key)?;
        let Some(etag) = prior else {
            return self.create(object, root, false);
        };
        let path = self
            .get(&object.key)?
            .ok_or_else(|| format!("channel pointer vanished: {}", object.key))?;
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read the standing channel pointer: {error}"))?;
        let _ = std::fs::remove_file(path);
        let current: Pointer = serde_json::from_str(&text)
            .map_err(|error| format!("cannot parse the standing channel pointer: {error}"))?;
        let draft = std::fs::read_to_string(&source)
            .map_err(|error| format!("cannot read the next channel pointer: {error}"))?;
        let next: Pointer = serde_json::from_str(&draft)
            .map_err(|error| format!("cannot parse the next channel pointer: {error}"))?;
        super::proof::advance(&current, &next)?;
        if current.version == next.version {
            if text == draft {
                return Ok(());
            }
            return Err(format!(
                "{} pointer drift at {}",
                next.channel, next.version
            ));
        }
        let output = self.put(
            object,
            &source,
            Rule {
                cache: "public, max-age=60, must-revalidate",
                header: "--if-match",
                value: &etag,
            },
        )?;
        if output.status.success() {
            Ok(())
        } else {
            Err(failure("activate stable", &object.key, &output))
        }
    }

    fn put(&self, object: &Local, source: &Path, rule: Rule<'_>) -> Result<Output, String> {
        self.command()
            .args(["put-object", "--bucket", self.held.bucket(), "--key"])
            .arg(&object.key)
            .arg("--body")
            .arg(source)
            .args([
                "--content-type",
                &object.remote.mime,
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
        if output.status.success() {
            let value: serde_json::Value =
                serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
            return value
                .get("ETag")
                .and_then(|held| held.as_str())
                .map(|held| Some(held.to_string()))
                .ok_or_else(|| format!("head-object returned no ETag for {key}"));
        }
        if absent(&output) {
            Ok(None)
        } else {
            Err(failure("head object", key, &output))
        }
    }

    fn get(&self, key: &str) -> Result<Option<PathBuf>, String> {
        let path = PathBuf::from(format!(
            ".plumb-s3-{}-{}",
            std::process::id(),
            key.replace('/', "-")
        ));
        let mut failed = None;
        for _ in 0..3 {
            let _ = std::fs::remove_file(&path);
            let output = self
                .command()
                .args(["get-object", "--bucket", self.held.bucket(), "--key", key])
                .arg(&path)
                .arg("--no-cli-pager")
                .output()
                .map_err(|error| format!("cannot run aws get-object: {error}"))?;
            if output.status.success() {
                return Ok(Some(path));
            }
            if absent(&output) {
                return Ok(None);
            }
            failed = Some(output);
        }
        let _ = std::fs::remove_file(&path);
        let output = failed.expect("a bounded object read always attempts once");
        Err(failure("get object", key, &output))
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

    fn fingerprint(&self) -> String {
        super::record::sha(self.held.endpoint().trim_end_matches('/').as_bytes())
    }

    fn same(&self, object: &Local) -> Result<bool, String> {
        let Some(path) = self.get(&object.key)? else {
            return Ok(false);
        };
        let same = super::record::digest(&path).is_ok_and(|(digest, size)| {
            digest == object.remote.sha256 && size == object.remote.size
        });
        let _ = std::fs::remove_file(path);
        Ok(same)
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
    let error = String::from_utf8_lossy(&output.stderr);
    format!("{action} {key}: {}", error.trim())
}
