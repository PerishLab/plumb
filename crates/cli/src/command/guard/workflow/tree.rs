use crate::catalog::set;
use crate::shape::workflow::Key;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
#[derive(Default)]
pub struct Tree {
    root: PathBuf,
    revision: Option<String>,
    leaves: BTreeMap<String, String>,
}
pub struct Git<'a>(&'a Path);
impl Tree {
    pub fn read(root: &Path, rev: Option<&str>) -> Result<Self, String> {
        let listed = match rev {
            Some(rev) => Git::new(root).listing(&["ls-tree", "-r", "-z", rev])?,
            None => Git::new(root).listing(&["ls-files", "--stage", "-z"])?,
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
        Ok(Self {
            root: root.to_path_buf(),
            revision: rev.map(str::to_string),
            leaves: held,
        })
    }
    pub fn selection(&self, roots: &[String]) -> Vec<(&String, &String)> {
        self.leaves
            .iter()
            .filter(|(path, _)| roots.iter().any(|root| covers(root, path)))
            .collect()
    }
    pub fn digest(&self, key: &Key) -> String {
        self.fingerprint(key)
            .expect("a staged Guard digest cannot fail")
    }
    fn fingerprint(&self, key: &Key) -> Result<String, String> {
        let mut sponge = Sha256::new();
        sponge.update(key.name().as_bytes());
        sponge.update([0]);
        for root in &key.roots {
            sponge.update(root.as_bytes());
            sponge.update([0]);
        }
        sponge.update([1]);
        sponge.update(set::current().release.forge.as_bytes());
        sponge.update([0]);
        for (path, meta) in self.selection(&key.paths) {
            if let Some(held) = super::release::fingerprint(
                key.lane() == "guard",
                (&self.root, self.revision.as_deref(), false),
                path,
                meta,
            )? {
                sponge.update(path.as_bytes());
                sponge.update([0]);
                sponge.update(held.as_bytes());
                sponge.update([0]);
            }
        }
        Ok(format!("{:x}", sponge.finalize()))
    }
    pub fn has(&self, path: &str) -> bool {
        self.leaves.contains_key(path)
    }
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.leaves.keys().map(String::as_str)
    }
    pub fn text(&self, path: &str) -> Result<Option<String>, String> {
        if !self.has(path) {
            return Ok(None);
        }
        Git::new(&self.root).file(self.revision.as_deref().unwrap_or(""), path)
    }
}
fn covers(root: &str, path: &str) -> bool {
    root == "*" || path == root || path.starts_with(&format!("{root}/"))
}
impl<'a> Git<'a> {
    pub fn new(root: &'a Path) -> Self {
        Self(root)
    }
    pub fn file(&self, rev: &str, path: &str) -> Result<Option<String>, String> {
        let object = if rev.is_empty() {
            format!(":{path}")
        } else {
            format!("{rev}:{path}")
        };
        let output = plumb::config::detached("git")
            .arg("-C")
            .arg(self.0)
            .args(["show", &object])
            .output()
            .map_err(|error| format!("cannot execute git: {error}"))?;
        if output.status.success() {
            return String::from_utf8(output.stdout)
                .map(Some)
                .map_err(|_| "git emitted non-UTF-8 output".to_string());
        }
        let error = String::from_utf8_lossy(&output.stderr);
        if error.contains("does not exist") || error.contains("exists on disk, but not in") {
            return Ok(None);
        }
        Err(error.trim().to_string())
    }
    fn listing(&self, args: &[&str]) -> Result<String, String> {
        let output = plumb::config::detached("git")
            .arg("-C")
            .arg(self.0)
            .args(args)
            .output()
            .map_err(|error| format!("cannot execute git: {error}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        String::from_utf8(output.stdout).map_err(|_| "git emitted non-UTF-8 output".to_string())
    }
}
