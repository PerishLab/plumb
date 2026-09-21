use sha2::{Digest, Sha256};
use std::sync::OnceLock;

pub mod v3;

pub struct Rules {
    files: Vec<(String, String)>,
    mark: String,
}

static CARRIED: OnceLock<Rules> = OnceLock::new();

pub fn carry(files: &'static [(&'static str, &'static str)]) {
    CARRIED.get_or_init(|| Rules::new(files));
}

pub fn rules() -> Result<&'static Rules, String> {
    CARRIED
        .get()
        .ok_or_else(|| "this binary carries no Plumb rules".to_string())
}

impl Rules {
    fn new(carried: &'static [(&'static str, &'static str)]) -> Self {
        let mut files = carried
            .iter()
            .map(|(path, body)| (path.to_string(), body.to_string()))
            .collect::<Vec<_>>();
        for (path, body) in overlay() {
            match files.iter_mut().find(|(held, _)| *held == path) {
                Some(held) => held.1 = body,
                None => files.push((path, body)),
            }
        }
        files.sort();
        let mut hasher = Sha256::new();
        for (path, body) in &files {
            hasher.update(path.as_bytes());
            hasher.update([0]);
            hasher.update(body.as_bytes());
            hasher.update([0]);
        }
        Self {
            files,
            mark: format!("{:x}", hasher.finalize()),
        }
    }

    pub fn read(&self, path: &str) -> Result<String, String> {
        self.files
            .iter()
            .find(|(held, _)| held == path)
            .map(|(_, body)| body.clone())
            .ok_or_else(|| format!("the carried Plumb rules hold no {path}"))
    }

    pub fn mark(&self) -> &str {
        &self.mark
    }
}

pub fn sha(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(debug_assertions)]
fn overlay() -> Vec<(String, String)> {
    let Some(home) = crate::config::value("PLUMB_HOME") else {
        return Vec::new();
    };
    let root = std::path::Path::new(&home).join("overlay");
    let mut held = Vec::new();
    walk(&root, &root, &mut held);
    held
}

#[cfg(debug_assertions)]
fn walk(root: &std::path::Path, dir: &std::path::Path, held: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, held);
        } else if let (Ok(name), Ok(body)) =
            (path.strip_prefix(root), std::fs::read_to_string(&path))
        {
            held.push((name.to_string_lossy().replace('\\', "/"), body));
        }
    }
}

#[cfg(not(debug_assertions))]
fn overlay() -> Vec<(String, String)> {
    Vec::new()
}
