use super::messages::Finding;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Component, Path};

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub(super) enum Reference {
    Bound {
        digest: String,
        files: BTreeMap<String, String>,
    },
    Unknown {
        reason: String,
    },
}

pub(super) struct Compiler<'a> {
    pub host: &'a str,
    pub all: bool,
    pub rustc: &'a str,
    pub clippy: &'a str,
}

impl Compiler<'_> {
    pub fn bind(&self, root: &Path, finding: &Finding) -> Reference {
        match (Identity {
            root,
            files: BTreeMap::new(),
        })
        .read(self, finding)
        {
            Ok(reference) => reference,
            Err(reason) => Reference::Unknown { reason },
        }
    }
}

pub(super) fn file(root: &Path, name: &str) -> Result<(String, String), String> {
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let path = root
        .join(name)
        .canonicalize()
        .map_err(|error| format!("cannot bind lock source {name}: {error}"))?;
    let relative = path
        .strip_prefix(&root)
        .map_err(|_| format!("lock source is outside the staged workspace: {name}"))?;
    let name = relative
        .components()
        .map(|part| match part {
            Component::Normal(part) => part.to_str().ok_or("lock source path is not UTF-8"),
            _ => Err("lock source path is not relative"),
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("/");
    if !path.is_file() || name.is_empty() {
        return Err("lock source is not a regular file".into());
    }
    let output = plumb::config::detached("git")
        .arg("-C")
        .arg(&root)
        .args(["ls-tree", "-z", "HEAD", "--"])
        .arg(format!(":(literal){name}"))
        .output()
        .map_err(|error| error.to_string())?;
    let entries = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();
    if !output.status.success()
        || entries.len() != 1
        || ![b"100644 blob ".as_slice(), b"100755 blob ".as_slice()]
            .iter()
            .any(|mode| entries[0].starts_with(mode))
    {
        return Err(format!(
            "lock source is not a tracked regular file in the compiled snapshot: {name}"
        ));
    }
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    Ok((name, format!("{:x}", Sha256::digest(bytes))))
}

struct Identity<'a> {
    root: &'a Path,
    files: BTreeMap<String, String>,
}

impl Identity<'_> {
    fn read(mut self, compiler: &Compiler<'_>, finding: &Finding) -> Result<Reference, String> {
        let package = finding
            .package
            .strip_prefix("path+file://")
            .and_then(|value| value.rsplit_once('#'))
            .map(|(_, package)| package)
            .filter(|package| !package.is_empty())
            .ok_or("lock reference is not a workspace package")?;
        let mut target = finding.target.clone();
        let path = target
            .get("src_path")
            .and_then(Value::as_str)
            .ok_or("lock target has no source path")?;
        let path = self.capture(path)?;
        self.manifest(&path)?;
        target["src_path"] = Value::String(path);
        let mut spans = finding.spans.clone();
        self.normalize(&mut spans)?;
        self.capture("Cargo.toml")?;
        self.capture("Cargo.lock")?;
        let basis = json!({"schema":"plumb.lock-reference/v1", "host":compiler.host, "all":compiler.all, "rustc":compiler.rustc, "clippy":compiler.clippy, "package":package, "target":target, "operation":finding.operation, "spans":spans, "files":self.files});
        let raw = serde_json::to_vec(&basis).map_err(|error| error.to_string())?;
        Ok(Reference::Bound {
            digest: format!("{:x}", Sha256::digest(raw)),
            files: self.files,
        })
    }

    fn capture(&mut self, name: &str) -> Result<String, String> {
        let (name, digest) = file(self.root, name)?;
        self.files.insert(name.clone(), digest);
        Ok(name)
    }

    fn manifest(&mut self, name: &str) -> Result<(), String> {
        let mut path = Path::new(name).parent();
        while let Some(directory) = path {
            let manifest = directory.join("Cargo.toml");
            if self.root.join(&manifest).is_file() {
                self.capture(manifest.to_str().ok_or("manifest path is not UTF-8")?)?;
                return Ok(());
            }
            path = directory.parent();
        }
        Err("lock source has no workspace Cargo.toml".into())
    }

    fn normalize(&mut self, value: &mut Value) -> Result<(), String> {
        match value {
            Value::Object(object) => {
                for (key, value) in object {
                    self.entry(key, value)?;
                }
            }
            Value::Array(array) => {
                for value in array {
                    self.normalize(value)?;
                }
            }
            _ => (),
        }
        Ok(())
    }
    fn entry(&mut self, key: &str, value: &mut Value) -> Result<(), String> {
        if key != "file_name" {
            return self.normalize(value);
        }
        let name = value.as_str().ok_or("compiler span has no file name")?;
        let name = self.capture(name)?;
        self.manifest(&name)?;
        *value = Value::String(name);
        Ok(())
    }
}
