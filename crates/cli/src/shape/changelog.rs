use semver::Version;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

pub use plumb::changelog::Proof;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pointer {
    schema: u32,
    channel: String,
    commit: String,
}

pub fn stamped(version: &str) -> String {
    match version.strip_prefix('v') {
        Some(_) => version.to_string(),
        None => format!("v{version}"),
    }
}

pub fn seat(workspace: &Path, version: &str) -> PathBuf {
    workspace
        .join("docs/CHANGELOG")
        .join(stamped(&base(version)))
}

pub fn artifacts(repository: &Path, version: &str) -> Result<Vec<PathBuf>, String> {
    let version = version.strip_prefix('v').unwrap_or(version);
    let parsed =
        Version::parse(version).map_err(|error| format!("invalid version {version}: {error}"))?;
    let home = seat(
        repository,
        &format!("{}.{}.{}", parsed.major, parsed.minor, parsed.patch),
    )
    .join("artifacts");
    let kind = match std::fs::symlink_metadata(&home) {
        Ok(held) if held.file_type().is_dir() => held,
        Ok(_) => {
            return Err(format!(
                "version artifact seat is not a directory: {}",
                home.display()
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("cannot inspect {}: {error}", home.display())),
    };
    if kind.file_type().is_symlink() {
        return Err(format!(
            "version artifact seat refuses symbolic link: {}",
            home.display()
        ));
    }
    let mut found = std::fs::read_dir(&home)
        .map_err(|error| format!("cannot read {}: {error}", home.display()))?
        .map(|entry| {
            let entry = entry.map_err(|error| error.to_string())?;
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            if !kind.is_file() || kind.is_symlink() {
                return Err(format!(
                    "version artifact is not a regular file: {}",
                    entry.path().display()
                ));
            }
            entry.file_name().to_str().ok_or_else(|| {
                format!(
                    "version artifact name is not UTF-8: {}",
                    entry.path().display()
                )
            })?;
            Ok(entry.path())
        })
        .collect::<Result<Vec<_>, String>>()?;
    found.sort();
    Ok(found)
}

pub fn previous(authority: &str) -> Result<Option<String>, String> {
    let url = format!(
        "{}/v1/channels/stable.json",
        authority.trim_end_matches('/')
    );
    let output = tempfile::NamedTempFile::new()
        .map_err(|error| format!("cannot stage previous stable pointer: {error}"))?;
    let response = Command::new("curl")
        .args(["--silent", "--show-error", "--location", "--output"])
        .arg(output.path())
        .args(["--write-out", "%{http_code}", &url])
        .output()
        .map_err(|error| format!("cannot execute curl: {error}"))?;
    if !response.status.success() {
        let error = String::from_utf8_lossy(&response.stderr).trim().to_string();
        return Err(format!(
            "cannot read previous stable pointer {url}: {}",
            if error.is_empty() {
                "curl failed"
            } else {
                &error
            }
        ));
    }
    let status = String::from_utf8_lossy(&response.stdout).trim().to_string();
    if status == "404" {
        return Ok(None);
    }
    if status != "200" {
        return Err(format!(
            "cannot read previous stable pointer {url}: HTTP {status}"
        ));
    }
    let bytes = std::fs::read(output.path())
        .map_err(|error| format!("cannot read previous stable pointer {url}: {error}"))?;
    let pointer: Pointer = serde_json::from_slice(&bytes)
        .map_err(|error| format!("cannot parse previous stable pointer {url}: {error}"))?;
    if pointer.schema != 1 || pointer.channel != "stable" || !commit(&pointer.commit) {
        return Err(format!(
            "previous stable pointer {url} has invalid identity"
        ));
    }
    Ok(Some(pointer.commit))
}

pub fn prove(root: &Path, home: &Path, version: &str, candidate: &str) -> Result<Proof, String> {
    let version = identity(version)?;
    let spec = super::super::dispatch::release::model::Spec::read(&root.join("plumb.toml"))?;
    let previous = previous(&spec.authority)?;
    plumb::changelog::prove(plumb::changelog::Claim {
        root,
        home,
        version: &version,
        previous: previous.as_deref(),
        candidate,
    })
}

fn base(version: &str) -> String {
    identity(version).unwrap_or_else(|_| version.trim_start_matches('v').to_string())
}

pub fn identity(version: &str) -> Result<String, String> {
    let parsed = Version::parse(version.trim_start_matches('v'))
        .map_err(|error| format!("invalid version {version}: {error}"))?;
    Ok(format!(
        "{}.{}.{}",
        parsed.major, parsed.minor, parsed.patch
    ))
}

fn commit(value: &str) -> bool {
    (40..=64).contains(&value.len())
        && value
            .chars()
            .all(|held| held.is_ascii_digit() || ('a'..='f').contains(&held))
}
