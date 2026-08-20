use semver::Version;
use std::path::{Path, PathBuf};
use std::process::Command;

pub use plumb::changelog::Proof;

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

pub fn prove(root: &Path, home: &Path, version: &str) -> Result<Proof, String> {
    let version = identity(version)?;
    let stamped = stamped(&version);
    let candidate = point(root, &stamped)?;
    let previous = prior(root, &version)?;
    plumb::changelog::prove(plumb::changelog::Claim {
        root,
        home,
        version: &version,
        previous: previous.as_deref(),
        candidate: &candidate,
    })
}

fn point(root: &Path, tag: &str) -> Result<String, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["rev-list", "-n", "1", tag])
        .output()
        .map_err(|error| format!("cannot run git rev-list: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{tag} names no point in this repository; a release note describes a version that shipped"
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn prior(root: &Path, version: &str) -> Result<Option<String>, String> {
    let held =
        Version::parse(version).map_err(|error| format!("invalid version {version}: {error}"))?;
    let output = Command::new("git")
        .current_dir(root)
        .args(["tag", "--list", "v*"])
        .output()
        .map_err(|error| format!("cannot run git tag: {error}"))?;
    let mut found = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| Version::parse(line.trim().trim_start_matches('v')).ok())
        .filter(|held| held.pre.is_empty())
        .filter(|other| other < &held)
        .collect::<Vec<_>>();
    found.sort();
    match found.pop() {
        Some(base) => point(root, &format!("v{base}")).map(Some),
        None => Ok(None),
    }
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
