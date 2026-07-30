use semver::VersionReq;
use serde_json::Value;
use std::path::Path;

const NAME: &str = "@perish/sealkit";
const SOURCE: &str = "jsr:@perish/sealkit";

pub struct Dependency {
    pub requirement: String,
    pub resolution: String,
}

pub enum Sealkit {
    Absent,
    Blind(String),
    Held(Dependency),
}

pub fn sealkit(root: &Path) -> Sealkit {
    let config = root.join(".runseal/deno.json");
    if !config.is_file() {
        return Sealkit::Absent;
    }
    let text = match std::fs::read_to_string(&config) {
        Ok(text) => text,
        Err(error) => {
            return Sealkit::Blind(format!("cannot read Sealkit requirement: {error}"));
        }
    };
    let doc: Value = match serde_json::from_str(&text) {
        Ok(doc) => doc,
        Err(_) => {
            return Sealkit::Blind(
                "cannot read Sealkit requirement: .runseal/deno.json has invalid JSON".into(),
            );
        }
    };
    let Some(imports) = doc.get("imports") else {
        return Sealkit::Absent;
    };
    let Some(imports) = imports.as_object() else {
        return Sealkit::Blind("cannot read Sealkit requirement: imports is not an object".into());
    };
    let Some(specifier) = imports.get(NAME) else {
        return Sealkit::Absent;
    };
    let Some(specifier) = specifier.as_str() else {
        return Sealkit::Blind(
            "cannot read Sealkit requirement: @perish/sealkit is not a string".into(),
        );
    };
    let Some(requirement) = requirement(specifier) else {
        return Sealkit::Held(Dependency {
            requirement: specifier.to_string(),
            resolution: String::new(),
        });
    };
    match resolution(root, &requirement) {
        Ok(resolution) => Sealkit::Held(Dependency {
            requirement,
            resolution,
        }),
        Err(error) => Sealkit::Blind(error),
    }
}

fn requirement(specifier: &str) -> Option<String> {
    let tail = specifier.strip_prefix(SOURCE)?;
    if tail.is_empty() {
        return Some("*".into());
    }
    let requirement = tail.strip_prefix('@')?;
    if requirement.is_empty() || requirement.contains('/') {
        return None;
    }
    Some(requirement.to_string())
}

fn resolution(root: &Path, requirement: &str) -> Result<String, String> {
    let path = root.join(".runseal/deno.lock");
    if !path.is_file() {
        return Err("cannot read Sealkit lock: .runseal/deno.lock is missing".into());
    }
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read Sealkit lock: {error}"))?;
    let doc: Value = serde_json::from_str(&text)
        .map_err(|_| "cannot read Sealkit lock: invalid JSON".to_string())?;
    let specifiers = doc
        .get("specifiers")
        .and_then(Value::as_object)
        .ok_or_else(|| "cannot read Sealkit lock: specifiers is not an object".to_string())?;
    let key = format!("{SOURCE}@{requirement}");
    if let Some(resolution) = specifiers.get(&key).and_then(Value::as_str) {
        return Ok(resolution.to_string());
    }
    direct(&doc, specifiers)
        .map_err(|error| format!("cannot read Sealkit lock: {error}; no resolution for {key}"))
}

fn direct(doc: &Value, specifiers: &serde_json::Map<String, Value>) -> Result<String, String> {
    let Some(workspace) = doc.get("workspace") else {
        return Err("workspace evidence is missing".into());
    };
    let dependencies = workspace
        .get("dependencies")
        .and_then(Value::as_array)
        .ok_or_else(|| "workspace dependencies are not an array".to_string())?;
    let prefix = format!("{SOURCE}@");
    let keys: Vec<_> = dependencies
        .iter()
        .filter_map(Value::as_str)
        .filter(|key| {
            key.strip_prefix(&prefix)
                .is_some_and(|requirement| VersionReq::parse(requirement).is_ok())
        })
        .collect();
    let [key] = keys.as_slice() else {
        return Err("workspace Sealkit requirement is absent or ambiguous".into());
    };
    specifiers
        .get(*key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("workspace Sealkit requirement has no resolution: {key}"))
}
