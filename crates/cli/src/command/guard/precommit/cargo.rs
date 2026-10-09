use crate::command::cache;
use std::fs::File;
use std::path::{Path, PathBuf};

pub(super) struct Lease {
    pub root: PathBuf,
    identity: String,
    since: u64,
    _lock: File,
}

impl Lease {
    pub fn new(root: &Path) -> Result<Self, String> {
        let identity = identity(root)?;
        let root = cache::seat(&cache::home()?, &identity);
        std::fs::create_dir_all(&root)
            .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
        let lock = cache::lease(&root)?;
        lock.lock()
            .map_err(|error| format!("cannot hold Guard compilation lease: {error}"))?;
        cache::adopt(&root, &identity)?;
        Ok(Self {
            root,
            identity,
            since: cache::now().saturating_sub(2),
            _lock: lock,
        })
    }

    pub fn settle(&self) {
        if let Err(error) = cache::settle(&self.root, &self.identity, self.since)
            .and_then(|_| cache::tour(&self.root))
        {
            eprintln!("plumb guard: Guard cache bookkeeping failed: {error}");
        }
    }
}

fn identity(root: &Path) -> Result<String, String> {
    let common = super::tree::git(
        root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        "resolve the guarded repository",
    )?;
    let common = std::fs::canonicalize(&common)
        .map_err(|error| format!("cannot resolve guarded repository {common}: {error}"))?;
    common
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| "guarded repository path is not UTF-8".to_string())
}
