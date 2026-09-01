use plumb::rig::Rig;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

pub(super) struct Inventory(Option<tempfile::NamedTempFile>);

impl Inventory {
    pub(super) fn fetch(url: &str) -> Result<Self, String> {
        if url.trim().is_empty() {
            return Ok(Self(None));
        }
        let file = tempfile::NamedTempFile::new()
            .map_err(|error| format!("cannot open an inventory seat: {error}"))?;
        let status = Command::new("curl")
            .args([
                "--fail",
                "--silent",
                "--show-error",
                "--location",
                "--connect-timeout",
                "3",
                "--max-time",
                "10",
                "--retry",
                "1",
                "--output",
            ])
            .arg(file.path())
            .arg(url)
            .status();
        Ok(match status {
            Ok(status) if status.success() => Self(Some(file)),
            _ => Self(None),
        })
    }

    pub(super) fn path(&self) -> Option<&Path> {
        self.0.as_ref().map(tempfile::NamedTempFile::path)
    }
}

pub(super) fn depot(root: &Path) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let held = crate::command::depot::held();
    let mark = held
        .mark()
        .filter(|mark| !mark.is_empty())
        .ok_or_else(|| format!("{} has no active depot identity", root.display()))?;
    Ok(format!("{} {mark}", rig.rules.channel))
}

pub(super) fn projection(root: &Path) -> String {
    let packages = root.join("packages");
    let manifest = std::fs::read_dir(packages)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path().join("package.json"))
        .find(|path| versioned(path));
    if versioned(&root.join("package.json")) {
        "package.json#/version".into()
    } else if let Some(manifest) = manifest {
        format!(
            "{}#/version",
            manifest.strip_prefix(root).unwrap_or(&manifest).display()
        )
    } else {
        "Cargo.toml#/workspace/package/version".into()
    }
}

pub(super) fn sources(spec: &crate::shape::release::Spec) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(&spec.root)
        .args(["ls-files", "-z", "--", "Cargo.toml", ":(glob)**/Cargo.toml"])
        .output()
        .map_err(|error| format!("cannot list Cargo manifests: {error}"))?;
    if !output.status.success() {
        return Err("cannot list Cargo manifests for the binary plan".into());
    }
    let listed = String::from_utf8(output.stdout)
        .map_err(|_| "Git listed a non-UTF-8 Cargo manifest".to_string())?;
    let mut roots = BTreeSet::new();
    for path in [
        ".cargo",
        "Cargo.lock",
        "Cargo.toml",
        "plumb.toml",
        "rust-toolchain",
        "rust-toolchain.toml",
    ] {
        if spec.root.join(path).exists() {
            roots.insert(path.to_string());
        }
    }
    for manifest in listed.split('\0').filter(|path| !path.is_empty()) {
        roots.insert(manifest.to_string());
        let seat = Path::new(manifest).parent().unwrap_or(Path::new(""));
        for path in [seat.join("src"), seat.join("build.rs")] {
            if spec.root.join(&path).exists() {
                roots.insert(display(&path));
            }
        }
    }
    if let Some(depends) = spec.depends.get("binary") {
        roots.extend(depends.iter().map(|path| display(path)));
    }
    Ok(roots.into_iter().collect())
}

fn display(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn versioned(path: &Path) -> bool {
    std::fs::read(path)
        .ok()
        .and_then(|body| serde_json::from_slice::<Value>(&body).ok())
        .and_then(|document| document["version"].as_str().map(str::to_string))
        .is_some()
}

pub(super) fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("ship plan row names no {key}"))
}

pub(super) fn strings<'a>(value: &'a Value, key: &str) -> Result<Vec<&'a str>, String> {
    value[key]
        .as_array()
        .ok_or_else(|| format!("ship plan row names no {key}"))?
        .iter()
        .map(|held| {
            held.as_str()
                .ok_or_else(|| format!("ship plan {key} is not text"))
        })
        .collect()
}

pub(super) fn object(value: &Value) -> Result<Map<String, Value>, String> {
    value
        .as_object()
        .cloned()
        .ok_or_else(|| "ship plan row is not an object".to_string())
}
