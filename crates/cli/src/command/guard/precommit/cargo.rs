use crate::command::cache;
use radicula_lock::Mode;
use std::fs::File;
use std::path::{Path, PathBuf};

pub(crate) struct Lease {
    pub root: PathBuf,
    identity: String,
    since: u64,
    _lock: File,
}

pub fn seat(root: &Path, tree: &str) -> Result<Option<PathBuf>, String> {
    let listed = super::tree::git(
        root,
        &["ls-tree", "--name-only", tree, "--", "Cargo.toml"],
        "read the staged tree",
    )?;
    if listed.is_empty() {
        return Ok(None);
    }
    Ok(Some(cache::seat(&cache::home()?, &identity(root)?)))
}

impl Lease {
    pub fn new(root: &Path) -> Result<Self, String> {
        let identity = identity(root)?;
        let root = cache::seat(&cache::home()?, &identity);
        if let Ok(metadata) = std::fs::symlink_metadata(&root)
            && (!metadata.is_dir() || metadata.file_type().is_symlink())
        {
            return Err(
                "Guard cache root is foreign; preserve or relocate it before retrying".into(),
            );
        }
        std::fs::create_dir_all(&root)
            .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
        let lock = cache::lease(&root)?;
        radicula_lock::lock(&lock, Mode::Exclusive)
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
        if let Err(error) = crate::diagnostic::measure("material.settle", || {
            cache::settle(&self.root, &self.identity, self.since)
                .and_then(|_| cache::tour(&self.root))
        }) {
            eprintln!("plumb guard: Guard cache bookkeeping failed: {error}");
        }
    }
}

pub(crate) fn identity(root: &Path) -> Result<String, String> {
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

pub(super) fn register(
    checks: &mut Vec<super::world::Check>,
    protection: Option<&crate::execution::lock::Protection>,
    check: super::world::Check,
) -> Result<(), String> {
    let native = protection
        .filter(|_| check.proof.name == "guard/rust")
        .map(|protection| {
            let execution = check
                .execution
                .as_deref()
                .ok_or("lock protection needs managed Cargo execution")?;
            let clippy = protection.clippy(execution)?;
            let material = plumb::depot::sha(
                &serde_json::to_vec(&("plumb.lock-world/v1", &check.material, clippy))
                    .map_err(|error| error.to_string())?,
            );
            let input = protection.bind("guard/lock", &check.proof.input);
            let world = plumb::depot::sha(
                &serde_json::to_vec(&(&material, &input)).map_err(|error| error.to_string())?,
            );
            Ok::<_, String>(super::world::Check {
                proof: plumb::guard::Action {
                    name: "guard/lock".into(),
                    input,
                    world,
                },
                material,
                commands: Vec::new(),
                exclusions: Vec::new(),
                execution: check.execution.clone(),
                protection: Some(protection.clone()),
            })
        })
        .transpose()?;
    checks.push(check);
    checks.extend(native);
    Ok(())
}
