use base64::Engine as _;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Command;

pub const SCHEMA: &str = "wharf.yard/v1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Good {
    pub path: String,
    pub sha256: String,
    pub executable: bool,
    pub body: String,
}

#[derive(Serialize)]
pub struct Consignment<'a> {
    pub schema: &'static str,
    pub repository: &'a str,
    pub marker: &'a str,
    pub kind: &'a str,
    pub objects: &'a [Good],
}

impl Consignment<'_> {
    pub fn encode(&self) -> Result<(Vec<u8>, String), String> {
        let body = serde_json::to_vec(self)
            .map_err(|error| format!("cannot encode the consignment: {error}"))?;
        let digest = format!("{:x}", Sha256::digest(&body));
        Ok((body, digest))
    }
}

pub fn good(path: String, body: &[u8], executable: bool) -> Good {
    Good {
        path,
        sha256: format!("{:x}", Sha256::digest(body)),
        executable,
        body: base64::engine::general_purpose::STANDARD.encode(body),
    }
}

pub fn directory(root: &Path) -> Result<Vec<Good>, String> {
    let mut held = Vec::new();
    walk(root, root, &mut held)?;
    if held.is_empty() {
        return Err(format!("{} holds nothing to consign", root.display()));
    }
    held.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(held)
}

fn walk(root: &Path, at: &Path, held: &mut Vec<Good>) -> Result<(), String> {
    let entries =
        std::fs::read_dir(at).map_err(|error| format!("cannot read {}: {error}", at.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("cannot read {}: {error}", at.display()))?
            .path();
        let kind = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
        if kind.is_dir() {
            walk(root, &path, held)?;
            continue;
        }
        if !kind.is_file() {
            return Err(format!("{} is not a plain file", path.display()));
        }
        let body = std::fs::read(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let relative = path
            .strip_prefix(root)
            .map_err(|_| format!("{} escapes {}", path.display(), root.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        held.push(good(relative, &body, executable(&kind)));
    }
    Ok(())
}

#[cfg(unix)]
fn executable(held: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    held.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn executable(_: &std::fs::Metadata) -> bool {
    false
}

pub struct Tree<'a>(pub &'a Path);

impl Tree<'_> {
    pub fn carried(&self, marker: &str, product: &str) -> Result<Vec<Good>, String> {
        let seat = format!("skills/{product}");
        let listed = self.git(&["ls-tree", "-r", &format!("refs/tags/{marker}"), "--", &seat])?;
        let mut held = Vec::new();
        for line in String::from_utf8_lossy(&listed).lines() {
            let (head, path) = line
                .split_once('\t')
                .ok_or_else(|| format!("git ls-tree answered {line:?}"))?;
            let fields = head.split_whitespace().collect::<Vec<_>>();
            let [mode @ ("100644" | "100755"), "blob", object] = fields.as_slice() else {
                return Err(format!("{path} at {marker} is not a plain file"));
            };
            let body = self.git(&["cat-file", "blob", object])?;
            let relative = path.strip_prefix(&format!("{seat}/")).unwrap_or(path);
            held.push(good(relative.to_string(), &body, *mode == "100755"));
        }
        if held.is_empty() {
            return Err(format!("{marker} carries no {seat}"));
        }
        Ok(held)
    }

    fn git(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.0)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))?;
        if output.status.success() {
            return Ok(output.stdout);
        }
        Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
