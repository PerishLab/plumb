use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Evidence {
    pub unbound: Vec<String>,
    pub blind: Vec<String>,
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    #[serde(rename = "manifest_path")]
    manifest: PathBuf,
    targets: Vec<Target>,
}

#[derive(Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
}

pub fn read(root: &Path) -> Evidence {
    let mut found = Evidence::default();
    if !root.join("Cargo.toml").is_file() {
        return found;
    }
    let metadata = match metadata(root) {
        Ok(metadata) => metadata,
        Err(error) => {
            found.blind.push(error);
            return found;
        }
    };
    for package in metadata.packages {
        let binaries = package
            .targets
            .iter()
            .filter(|target| target.kind.iter().any(|kind| kind == "bin"))
            .map(|target| target.name.as_str())
            .collect::<Vec<_>>();
        if binaries.is_empty() {
            continue;
        }
        let source = package.manifest.with_file_name("src");
        let mut text = String::new();
        if let Err(error) = gather(&source, &mut text) {
            found.blind.push(format!(
                "cannot read sources of crate {}: {error}",
                package.name
            ));
            continue;
        }
        if invoked(&text, "version!") && !invoked(&text, "identity!") {
            found.unbound.push(format!(
                "crate {} reads plumb::version! but never invokes plumb::identity!, so binaries {} carry no identity region",
                package.name,
                binaries.join(", ")
            ));
        }
    }
    found.unbound.sort();
    found
}

fn metadata(root: &Path) -> Result<Metadata, String> {
    let output = crate::cargo::command()
        .args(["metadata", "--frozen", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot read Cargo targets: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot read Cargo targets: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cannot read Cargo metadata: {error}"))
}

fn gather(dir: &Path, text: &mut String) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            gather(&path, text)?;
        } else if path.extension().is_some_and(|kind| kind == "rs") {
            text.push_str(&std::fs::read_to_string(&path)?);
            text.push('\n');
        }
    }
    Ok(())
}

fn invoked(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(at, _)| {
        text[..at]
            .bytes()
            .last()
            .is_none_or(|byte| !(byte.is_ascii_alphanumeric() || byte == b'_'))
    })
}
