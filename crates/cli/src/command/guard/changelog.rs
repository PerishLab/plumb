use semver::Version;
use std::path::Path;
use std::process::Command;

pub(crate) use plumb::changelog::Proof;

pub fn stamped(version: &str) -> String {
    match version.strip_prefix('v') {
        Some(_) => version.to_string(),
        None => format!("v{version}"),
    }
}

pub fn prove(root: &Path, home: &Path, version: &str) -> Result<Proof, String> {
    let version = identity(version)?;
    let stamped = stamped(&version);
    let candidate = point(root, &stamped)?;
    let previous = prior(root, &version, &candidate)?;
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

fn prior(root: &Path, version: &str, candidate: &str) -> Result<Option<String>, String> {
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
    while let Some(base) = found.pop() {
        let commit = point(root, &format!("v{base}"))?;
        let status = Command::new("git")
            .current_dir(root)
            .args(["merge-base", "--is-ancestor", &commit, candidate])
            .status()
            .map_err(|error| format!("cannot run git merge-base: {error}"))?;
        if status.success() {
            return Ok(Some(commit));
        }
    }
    Ok(None)
}

pub fn identity(version: &str) -> Result<String, String> {
    let parsed = Version::parse(version.trim_start_matches('v'))
        .map_err(|error| format!("invalid version {version}: {error}"))?;
    Ok(format!(
        "{}.{}.{}",
        parsed.major, parsed.minor, parsed.patch
    ))
}
