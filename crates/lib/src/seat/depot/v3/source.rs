use std::collections::BTreeMap;
use std::path::Path;

use super::{Identity, Manifest, Object};

#[derive(Debug)]
pub struct Bundle {
    pub manifest: Manifest,
    pub bodies: BTreeMap<String, Vec<u8>>,
}

impl Bundle {
    pub fn read(root: &Path, identity: Identity) -> Result<Self, String> {
        let metadata = std::fs::symlink_metadata(root)
            .map_err(|error| format!("cannot inspect depot source {}: {error}", root.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "depot source is a symbolic link: {}",
                root.display()
            ));
        }
        let base = root
            .canonicalize()
            .map_err(|error| format!("cannot resolve depot source {}: {error}", root.display()))?;
        if !metadata.is_dir() {
            return Err(format!(
                "depot source is not a directory: {}",
                root.display()
            ));
        }
        let mut bodies = BTreeMap::new();
        walk(&base, &base, &mut bodies)?;
        let objects = bodies
            .iter()
            .map(|(path, body)| {
                let file = base.join(path);
                Ok(Object {
                    path: path.clone(),
                    sha256: super::super::sha(body),
                    size: body.len() as u64,
                    media: media(path).to_string(),
                    executable: executable(&file)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let manifest = Manifest::new(identity, objects)?;
        Ok(Self { manifest, bodies })
    }
}

fn walk(root: &Path, at: &Path, bodies: &mut BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let mut entries = std::fs::read_dir(at)
        .map_err(|error| format!("cannot read depot source {}: {error}", at.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot read depot source {}: {error}", at.display()))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect depot source {}: {error}", path.display()))?;
        let kind = metadata.file_type();
        if kind.is_symlink() || (!kind.is_file() && !kind.is_dir()) {
            return Err(format!(
                "depot source carries a special object: {}",
                path.display()
            ));
        }
        if kind.is_dir() {
            walk(root, &path, bodies)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|error| format!("cannot anchor depot object {}: {error}", path.display()))?;
        let key = key(relative)?;
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("cannot read depot object {}: {error}", path.display()))?;
        bodies.insert(key, bytes);
    }
    Ok(())
}

fn key(path: &Path) -> Result<String, String> {
    path.components()
        .map(|part| {
            part.as_os_str()
                .to_str()
                .ok_or_else(|| format!("depot object path is not UTF-8: {}", path.display()))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("/"))
}

fn media(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|held| held.to_str()) {
        Some("json") => "application/json",
        Some("toml") => "application/toml",
        Some("yaml" | "yml") => "application/yaml",
        Some("md" | "txt") => "text/plain",
        Some("sh") => "text/x-shellscript",
        Some("ps1") => "text/x-powershell",
        _ => "application/octet-stream",
    }
}

#[cfg(unix)]
fn executable(path: &Path) -> Result<bool, String> {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|held| held.permissions().mode() & 0o111 != 0)
        .map_err(|error| format!("cannot inspect depot object {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn executable(path: &Path) -> Result<bool, String> {
    path.metadata()
        .map(|_| false)
        .map_err(|error| format!("cannot inspect depot object {}: {error}", path.display()))
}
