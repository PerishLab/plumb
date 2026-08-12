use semver::Version;
use std::path::{Path, PathBuf};

const TONGUES: [&str; 2] = ["en", "zh"];
const LEAVES: [&str; 2] = ["INDEX.md", "MIGRATION.md"];

pub fn stamped(version: &str) -> String {
    match version.strip_prefix('v') {
        Some(_) => version.to_string(),
        None => format!("v{version}"),
    }
}

pub fn seat(root: &Path, version: &str) -> PathBuf {
    root.join("docs/CHANGELOG").join(stamped(version))
}

pub fn artifacts(root: &Path, version: &str) -> Result<Vec<PathBuf>, String> {
    let version = version.strip_prefix('v').unwrap_or(version);
    let parsed =
        Version::parse(version).map_err(|error| format!("invalid version {version}: {error}"))?;
    let home = seat(
        root,
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

pub fn read(root: &Path, version: &str) -> Vec<String> {
    let home = seat(root, version);
    let mut found = Vec::new();
    for tongue in TONGUES {
        for leaf in LEAVES {
            if let Some(note) = judged(&home, tongue, leaf) {
                found.push(note);
            }
        }
    }
    found
}

fn judged(home: &Path, tongue: &str, leaf: &str) -> Option<String> {
    let shown = format!("{tongue}/{leaf}");
    match std::fs::read_to_string(home.join(tongue).join(leaf)) {
        Ok(text) if !text.trim().is_empty() => None,
        Ok(_) => Some(format!("{shown} is empty")),
        Err(_) => Some(format!("{shown} is missing")),
    }
}
