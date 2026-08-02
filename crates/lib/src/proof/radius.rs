use semver::Version;
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "plumb.radius/v1";

pub struct Request<'a> {
    pub roots: &'a [PathBuf],
    pub product: &'a str,
    pub candidate: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Seat {
    pub root: PathBuf,
    pub ecosystem: &'static str,
    pub lock: String,
    pub resolution: String,
    pub behind: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Blind {
    pub root: PathBuf,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub product: String,
    pub candidate: String,
    pub seats: Vec<Seat>,
    pub behind: usize,
    pub blind: Vec<Blind>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Refusal {
    pub kind: &'static str,
    pub message: String,
}

impl Display for Refusal {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for Refusal {}

fn refuse(kind: &'static str, message: impl Into<String>) -> Refusal {
    Refusal {
        kind,
        message: message.into(),
    }
}

pub fn check(request: Request<'_>) -> Result<Report, Refusal> {
    let product = request.product.trim();
    if product.is_empty() {
        return Err(refuse("product", "product is required"));
    }
    let candidate = version(request.candidate)?;
    let mut seats = Vec::new();
    let mut blind = Vec::new();
    for root in request.roots {
        match survey(root, product, &candidate) {
            Ok(found) => seats.extend(found),
            Err(reason) => blind.push(Blind {
                root: root.clone(),
                reason,
            }),
        }
    }
    seats.sort_by(|held, other| {
        held.root
            .cmp(&other.root)
            .then_with(|| held.lock.cmp(&other.lock))
    });
    let behind = seats.iter().filter(|seat| seat.behind).count();
    Ok(Report {
        schema: SCHEMA,
        product: product.to_string(),
        candidate: candidate.to_string(),
        seats,
        behind,
        blind,
    })
}

fn version(raw: &str) -> Result<Version, Refusal> {
    let held = raw.trim().strip_prefix('v').unwrap_or(raw.trim());
    if held.is_empty() {
        return Err(refuse("candidate", "candidate version is required"));
    }
    Version::parse(held).map_err(|error| {
        refuse(
            "candidate",
            format!("invalid candidate version {raw}: {error}"),
        )
    })
}

fn survey(root: &Path, product: &str, candidate: &Version) -> Result<Vec<Seat>, String> {
    let seen = std::fs::canonicalize(root)
        .map_err(|error| format!("cannot resolve {}: {error}", root.display()))?;
    if !seen.is_dir() {
        return Err(format!("{} is not a directory", seen.display()));
    }
    let mut found = Vec::new();
    for (name, ecosystem) in [("Cargo.lock", "cargo"), ("deno.lock", "jsr")] {
        let path = seen.join(name);
        if !path.exists() {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|error| format!("cannot read {name}: {error}"))?;
        let resolved = match ecosystem {
            "cargo" => cargo(&bytes, product)?,
            _ => jsr(&bytes, product)?,
        };
        for resolution in resolved {
            let held = Version::parse(&resolution).map_err(|error| {
                format!(
                    "{name}: {product} resolves to {resolution}, which is not a version: {error}"
                )
            })?;
            found.push(Seat {
                root: seen.clone(),
                ecosystem,
                lock: name.to_string(),
                behind: held < *candidate,
                resolution,
            });
        }
    }
    Ok(found)
}

pub fn cargo(bytes: &[u8], product: &str) -> Result<Vec<String>, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| format!("Cargo.lock: {error}"))?;
    let document: toml::Value =
        toml::from_str(text).map_err(|error| format!("Cargo.lock: {error}"))?;
    let mut found = Vec::new();
    for entry in document
        .get("package")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
    {
        let named = entry.get("name").and_then(toml::Value::as_str);
        if named != Some(product) {
            continue;
        }
        let held = entry
            .get("version")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("Cargo.lock: package {product} has no version"))?;
        found.push(held.to_string());
    }
    Ok(found)
}

pub fn jsr(bytes: &[u8], product: &str) -> Result<Vec<String>, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| format!("deno.lock: {error}"))?;
    let document: serde_json::Value =
        serde_json::from_str(text).map_err(|error| format!("deno.lock: {error}"))?;
    let mut found = Vec::new();
    let sealed = format!("{product}@");
    if let Some(table) = document.get("jsr").and_then(serde_json::Value::as_object) {
        for key in table.keys() {
            if let Some(rest) = key.strip_prefix(&sealed) {
                found.push(rest.to_string());
            }
        }
    }
    let wanted = format!("jsr:{product}@");
    for (key, value) in document
        .get("specifiers")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flatten()
    {
        if !key.starts_with(&wanted) {
            continue;
        }
        let held = value
            .as_str()
            .ok_or_else(|| format!("deno.lock: {key} has no resolved version"))?;
        found.push(held.to_string());
    }
    found.sort();
    found.dedup();
    Ok(found)
}
