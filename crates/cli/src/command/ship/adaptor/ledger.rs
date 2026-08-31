use crate::shape::release::{Cargo, Spec};
use semver::Version;
use serde::Deserialize;
use std::process::Command;
use std::time::Duration;

#[derive(Deserialize)]
pub struct Entry {
    vers: String,
    cksum: String,
    #[serde(default)]
    yanked: bool,
}

pub struct Readback<'a> {
    pub spec: &'a Spec,
    pub cargo: &'a Cargo,
    pub package: &'a str,
    pub version: &'a Version,
    pub checksum: &'a str,
    pub token: &'a str,
}

pub struct Presence<'a> {
    pub spec: &'a Spec,
    pub cargo: &'a Cargo,
    pub package: &'a str,
    pub version: &'a Version,
    pub token: &'a str,
}

pub fn entries(
    spec: &Spec,
    cargo: &Cargo,
    package: &str,
    token: &str,
) -> Result<Vec<Entry>, String> {
    let index = index(spec, &cargo.registry)?;
    let output = Command::new("curl")
        .args(["--silent", "--show-error", "--location"])
        .args(["--header", &format!("Authorization: {token}")])
        .args(["--write-out", "\n%{http_code}"])
        .arg(format!("{index}/{}", route(package)))
        .output()
        .map_err(|error| format!("cannot read Cargo registry: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Cargo registry read failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    let (body, status) = text
        .rsplit_once('\n')
        .ok_or_else(|| "Cargo registry response has no status".to_string())?;
    if status == "404" {
        return Ok(Vec::new());
    }
    if status != "200" {
        return Err(format!("Cargo registry readback returned HTTP {status}"));
    }
    body.lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).map_err(|error| error.to_string()))
        .collect()
}

pub fn verified(
    package: &str,
    wanted: &Version,
    checksum: &str,
    entries: &[Entry],
) -> Result<bool, String> {
    for entry in entries {
        let held = Version::parse(&entry.vers)
            .map_err(|error| format!("invalid registry version {}: {error}", entry.vers))?;
        if held > *wanted {
            return Err(format!(
                "registry {package} {} is ahead of target {wanted}",
                entry.vers
            ));
        }
    }
    let Some(found) = entries
        .iter()
        .find(|entry| entry.vers == wanted.to_string())
    else {
        return Ok(false);
    };
    if found.yanked || found.cksum != checksum {
        return Err(format!(
            "registry {package} {wanted} is not the packaged crate"
        ));
    }
    Ok(true)
}

pub fn readback(input: Readback<'_>) -> Result<(), String> {
    for attempt in 0..12 {
        if verified(
            input.package,
            input.version,
            input.checksum,
            &entries(input.spec, input.cargo, input.package, input.token)?,
        )? {
            return Ok(());
        }
        if attempt < 11 {
            std::thread::sleep(Duration::from_secs(5));
        }
    }
    Err(format!(
        "registry did not expose {} {}",
        input.package, input.version
    ))
}

pub(in crate::command) fn present(input: Presence<'_>) -> Result<(), String> {
    let found = entries(input.spec, input.cargo, input.package, input.token)?
        .into_iter()
        .any(|entry| !entry.yanked && entry.vers == input.version.to_string());
    if found {
        Ok(())
    } else {
        Err(format!(
            "Cargo registry carries no {} {}",
            input.package, input.version
        ))
    }
}

fn index(spec: &Spec, registry: &str) -> Result<String, String> {
    let path = spec.root.join(".cargo/config.toml");
    let document = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let value: toml::Value = toml::from_str(&document).map_err(|error| error.to_string())?;
    value
        .get("registries")
        .and_then(|held| held.get(registry))
        .and_then(|held| held.get("index"))
        .and_then(toml::Value::as_str)
        .and_then(|held| held.strip_prefix("sparse+"))
        .map(|held| held.trim_end_matches('/').to_string())
        .ok_or_else(|| format!("missing sparse registry {registry} in {}", path.display()))
}

pub(in crate::command::ship) fn publication(
    spec: &Spec,
    cargo: &Cargo,
    package: &str,
) -> Result<String, String> {
    Ok(format!(
        "{}/{}",
        index(spec, &cargo.registry)?,
        route(package)
    ))
}

fn route(name: &str) -> String {
    let name = name.to_ascii_lowercase();
    match name.len() {
        1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{}", &name[0..1], name),
        _ => format!("{}/{}/{}", &name[0..2], &name[2..4], name),
    }
}
