use crate::catalog::set::RULES;
use crate::shape::workflow::Key;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

pub struct Tree(BTreeMap<String, String>);

impl Tree {
    pub fn read(root: &Path, rev: Option<&str>) -> Result<Self, String> {
        let listed = match rev {
            Some(rev) => listing(root, &["ls-tree", "-r", "-z", rev])?,
            None => listing(root, &["ls-files", "--stage", "-z"])?,
        };
        let mut held = BTreeMap::new();
        for record in listed.split('\0').filter(|record| !record.is_empty()) {
            let (meta, path) = record
                .split_once('\t')
                .ok_or_else(|| "git emitted a malformed entry".to_string())?;
            let mut fields = meta.split(' ');
            let mode = fields.next().unwrap_or_default();
            let oid = match rev {
                Some(_) => fields.nth(1).unwrap_or_default(),
                None => fields.next().unwrap_or_default(),
            };
            if mode.is_empty() || oid.is_empty() {
                return Err("git emitted a malformed entry".to_string());
            }
            held.insert(path.to_string(), format!("{mode} {oid}"));
        }
        Ok(Self(held))
    }

    fn under(&self, roots: &[String]) -> Vec<(&String, &String)> {
        self.0
            .iter()
            .filter(|(path, _)| roots.iter().any(|root| covers(root, path)))
            .collect()
    }

    pub fn digest(&self, key: &Key) -> String {
        let mut sponge = Sha256::new();
        sponge.update(key.name().as_bytes());
        sponge.update([0]);
        for root in &key.roots {
            sponge.update(root.as_bytes());
            sponge.update([0]);
        }
        sponge.update([1]);
        sponge.update(RULES.release.forge.as_bytes());
        sponge.update([0]);
        for (path, meta) in self.under(&key.paths) {
            sponge.update(path.as_bytes());
            sponge.update([0]);
            sponge.update(meta.as_bytes());
            sponge.update([0]);
        }
        format!("{:x}", sponge.finalize())
    }

    pub fn covered(&self, key: &Key) -> usize {
        self.under(&key.paths).len()
    }
}

fn covers(root: &str, path: &str) -> bool {
    root == "*" || path == root || path.starts_with(&format!("{root}/"))
}

pub fn history(root: &Path, rev: &str) -> Result<Vec<String>, String> {
    let range = format!("{rev}..HEAD");
    let listed = listing(root, &["rev-list", "--reverse", &range])?;
    Ok(listed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

fn listing(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| format!("cannot execute git: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    String::from_utf8(output.stdout).map_err(|_| "git emitted non-UTF-8 output".to_string())
}
