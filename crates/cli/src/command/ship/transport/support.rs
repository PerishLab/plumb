use serde_json::{Map, Value};
use std::path::Path;
use std::process::Command;

pub(super) struct Plan<'a> {
    pub stage: &'a str,
    pub action: &'a str,
    pub projections: &'a [&'a str],
    pub roots: &'a [&'a str],
    pub runner: &'a str,
    pub workload: Option<&'a str>,
    pub release: Option<&'a str>,
    pub target: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Contract {
    Portable,
    Version,
    Exact,
}

pub(super) fn contract(action: &str) -> Contract {
    match action {
        "ship/cargo" => Contract::Version,
        "ship/oci" => Contract::Portable,
        "ship/cfworker" => Contract::Exact,
        _ => Contract::Portable,
    }
}

pub(super) fn embedded(action: &str) -> bool {
    action == "ship/cargo"
}

pub(super) fn authority(
    held: &plumb::rig::Authority,
    product: &str,
) -> Result<plumb::rig::Authority, String> {
    let bucket = format!("perish-{product}-releases");
    if !held.bucket.is_empty() && held.bucket != bucket {
        return Err(format!(
            "release publish authority must target derived bucket {bucket}"
        ));
    }
    let actual = crate::command::release::storage::fingerprint(&held.endpoint);
    if actual != held.fingerprint {
        return Err(format!(
            "release publish authority fingerprint drift: expected {}, got {actual}",
            held.fingerprint
        ));
    }
    let mut derived = held.clone();
    derived.bucket = bucket;
    Ok(derived)
}

pub(super) struct Inventory(Option<tempfile::NamedTempFile>);

impl Inventory {
    pub(super) fn fetch(url: &str) -> Result<Self, String> {
        if url.trim().is_empty() {
            return Ok(Self(None));
        }
        let file = tempfile::NamedTempFile::new()
            .map_err(|error| format!("cannot open an inventory seat: {error}"))?;
        let output = Command::new("curl")
            .args([
                "--silent",
                "--show-error",
                "--location",
                "--connect-timeout",
                "3",
                "--max-time",
                "10",
                "--retry",
                "1",
                "--write-out",
                "%{http_code}",
                "--output",
            ])
            .arg(file.path())
            .arg(url)
            .output()
            .map_err(|error| format!("cannot fetch Ship inventory: {error}"))?;
        if !output.status.success() {
            return Err("cannot fetch Ship inventory: transport failed".into());
        }
        match String::from_utf8_lossy(&output.stdout).trim() {
            "200" => Ok(Self(Some(file))),
            "404" => Ok(Self(None)),
            status => Err(format!("cannot fetch Ship inventory: HTTP {status}")),
        }
    }

    pub(super) fn path(&self) -> Option<&Path> {
        self.0.as_ref().map(tempfile::NamedTempFile::path)
    }
}

pub(super) fn projection() -> String {
    "Cargo.toml#/workspace/package/version".into()
}

pub(super) fn sources(spec: &crate::shape::release::Spec) -> Result<Vec<String>, String> {
    let mut roots = super::sources::read(&spec.root)?;
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

pub(super) fn matrix(include: Vec<Value>) -> Value {
    if include.is_empty() {
        idle()
    } else {
        serde_json::json!({ "include": include })
    }
}

pub(super) fn idle() -> Value {
    serde_json::json!({ "include": [{ "runner": "docker", "control": "reuse" }] })
}

pub(super) fn carry(request: &mut Map<String, Value>, workloads: &[Value]) -> Result<(), String> {
    request
        .get_mut("operation")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "ship plan carries no operation".to_string())?
        .insert("workloads".into(), Value::Array(workloads.to_vec()));
    Ok(())
}

pub(super) fn pnpm(root: &std::path::Path, install: bool) -> Result<(), String> {
    if !install || !root.join("pnpm-lock.yaml").is_file() {
        return Ok(());
    }
    let store = root.join("target/pnpm-store");
    command(
        root,
        "pnpm",
        &[
            "install",
            "--frozen-lockfile",
            "--store-dir",
            &store.to_string_lossy(),
        ],
    )
}

fn command(root: &std::path::Path, program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|error| format!("cannot run {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{program} failed while materializing a ship request"
        ))
    }
}
