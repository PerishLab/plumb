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
    depot: &str,
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
                &serde_json::to_vec(&(&material, depot, &input))
                    .map_err(|error| error.to_string())?,
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
                brand: None,
            })
        })
        .transpose()?;
    checks.push(check);
    checks.extend(native);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{Selection, tree::Index, world::Prepared};
    use serde::{Deserialize, Serialize};
    use std::{env, fs, path::Path, process::Command};

    #[derive(Deserialize, Serialize)]
    struct Snapshot {
        depot: String,
        material: String,
        plumb: String,
        lock: String,
        held: String,
    }

    fn inspect(root: &Path) -> Snapshot {
        plumb::depot::carry(crate::catalog::carried::FILES);
        crate::catalog::prepare().unwrap();
        crate::catalog::set::prepare().unwrap();
        let tree = plumb::guard::tree(root).unwrap();
        let index = Index::new(root, &tree).unwrap();
        let seat = super::seat(root, &tree).unwrap();
        let prepared = Prepared::new(root, &index, Selection::Local, seat.as_deref()).unwrap();
        let find = |name| {
            prepared
                .checks
                .iter()
                .find(|check| check.proof.name == name)
                .unwrap()
        };
        let (plumb, lock) = (find("guard/plumb"), find("guard/lock"));
        let environment = &plumb.execution.as_ref().unwrap().environment;
        assert!(environment.get("PLUMB_DEPOT_MARK").is_none());
        Snapshot {
            depot: prepared.depot,
            material: prepared.material,
            plumb: plumb.proof.world.clone(),
            lock: lock.proof.world.clone(),
            held: lock.material.clone(),
        }
    }

    fn child(home: &Path, root: &Path, name: &str) -> Snapshot {
        let test = "command::guard::precommit::cargo::tests::depot";
        let result = Command::new(env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env("PLUMB_CARGO_TEST_ROOT", root)
            .env("PLUMB_CARGO_TEST_NAME", name)
            .env("PLUMB_HOME", home)
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(result.status.success(), "{stderr}");
        serde_json::from_slice(&fs::read(home.join(format!("{name}.json"))).unwrap()).unwrap()
    }

    #[test]
    fn depot() {
        if let (Some(root), Some(name)) = (
            env::var_os("PLUMB_CARGO_TEST_ROOT"),
            env::var_os("PLUMB_CARGO_TEST_NAME"),
        ) {
            let snapshot = inspect(Path::new(&root));
            let home = env::var_os("PLUMB_HOME").unwrap();
            let out = Path::new(&home).join(format!("{}.json", name.to_string_lossy()));
            fs::write(out, serde_json::to_vec(&snapshot).unwrap()).unwrap();
            return;
        }
        let (home, root) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let (home, root) = (home.path(), root.path());
        let git = |args: &[&str]| super::super::tree::git(root, args, "cargo fixture").unwrap();
        git(&["init", "-q"]);
        git(&[
            "remote",
            "add",
            "origin",
            "https://github.com/Example/probe.git",
        ]);
        fs::create_dir(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='probe'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        fs::write(
            root.join("plumb.toml"),
            "[identity]\nns='plumb'\n[guard.lock]\nenforce=true\ntarget='x86_64-unknown-linux-gnu'\nfeatures=['default']\n[workflow.hash.guard]\nrust=['Cargo.toml','Cargo.lock','src']\nplumb=['Cargo.toml','Cargo.lock','src']\nectropy=['ectropy.toml','src']\n",
        )
        .unwrap();
        fs::write(
            root.join("ectropy.toml"),
            "[[ban]]\nsyntax='lock'\npaths=['src/**/*.rs']\n",
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn answer() -> u8 { 1 }\n").unwrap();
        let mut cargo = Command::new("cargo");
        cargo
            .args(["generate-lockfile", "--offline"])
            .current_dir(root);
        assert!(cargo.status().unwrap().success());
        git(&["add", "."]);
        let first = child(home, root, "base");
        fs::create_dir_all(home.join("overlay")).unwrap();
        fs::write(home.join("overlay/probe.txt"), "overlay\n").unwrap();
        let next = child(home, root, "overlay");
        assert_ne!(first.depot, next.depot);
        assert_ne!(first.plumb, next.plumb);
        assert_ne!(first.lock, next.lock);
        assert_eq!(first.held, next.held);
        assert_eq!(first.material, next.material);
    }
}
