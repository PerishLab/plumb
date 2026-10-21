pub use super::claim::Claim;
use super::owner;
use std::collections::BTreeSet;

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

pub const PREFIX: &str = "guard-";
pub(super) const RETAINED: &str = "guard-source-";

const AGE: Duration = Duration::from_secs(3600);

static LIVE: Mutex<Vec<(PathBuf, PathBuf)>> = Mutex::new(Vec::new());

pub fn root() -> Option<PathBuf> {
    crate::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| crate::config::data("plumb"))
        .map(|home| home.join("tmp"))
}

pub fn contains(root: &Path, path: &Path) -> bool {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let path = path.canonicalize().unwrap_or_else(|_| {
        path.parent()
            .and_then(|parent| parent.canonicalize().ok())
            .zip(path.file_name())
            .map(|(parent, name)| parent.join(name))
            .unwrap_or_else(|| path.to_path_buf())
    });
    path.parent() == Some(root.as_path())
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(PREFIX))
}

pub fn held() -> bool {
    !live().is_empty()
}

pub fn abandon() {
    for (source, root) in live().drain(..) {
        if owner::Owner::retained(&root)
            && !owner::Owner::read(&root.with_extension("lock"))
                .is_ok_and(|owner| owner.owns(&root))
        {
            continue;
        }
        let _ = crate::config::detached("git")
            .arg("-C")
            .arg(&source)
            .args(["worktree", "remove", "--force"])
            .arg(&root)
            .output();
        if !owner::Owner::retained(&root) {
            let _ = std::fs::remove_file(root.with_extension("lock"));
        }
    }
}

pub fn reap(base: &Path) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    let trees = entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with(PREFIX))
        .map(|name| base.join(name.strip_suffix(".lock").unwrap_or(&name)))
        .collect::<BTreeSet<_>>();
    for tree in trees {
        sweep(&tree);
    }
}

fn sweep(tree: &Path) {
    let path = tree.with_extension("lock");
    if owner::Owner::retained(tree)
        && (!owner::Owner::read(&path).is_ok_and(|owner| owner.matches(tree)) || young(&path))
    {
        return;
    }
    if !path.exists() && young(tree) {
        return;
    }
    let Ok(lock) = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
    else {
        return;
    };
    if lock.try_lock().is_err() || !same(&path, &lock) {
        return;
    }
    if owner::Owner::retained(tree)
        && !owner::Owner::read(&path).is_ok_and(|owner| owner.matches(tree))
    {
        return;
    }
    remove(tree);
    let _ = std::fs::remove_file(&path);
}

fn young(tree: &Path) -> bool {
    std::fs::metadata(tree)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_none_or(|age| age < AGE)
}

fn remove(tree: &Path) {
    let common = std::fs::read_to_string(tree.join(".git"))
        .ok()
        .and_then(|link| {
            link.trim()
                .strip_prefix("gitdir:")
                .map(|admin| tree.join(admin.trim()))
        })
        .and_then(|admin| admin.parent().and_then(Path::parent).map(Path::to_path_buf));
    if let Some(common) = &common {
        git(common, &["worktree", "remove", "--force"], Some(tree));
    }
    let _ = std::fs::remove_dir_all(tree);
    if let Some(common) = &common {
        git(common, &["worktree", "prune"], None);
    }
}

fn git(common: &Path, args: &[&str], tree: Option<&Path>) {
    let mut command = crate::config::detached("git");
    command.arg("--git-dir").arg(common).args(args);
    if let Some(tree) = tree {
        command.arg(tree);
    }
    let _ = command.output();
}

pub(super) fn live() -> MutexGuard<'static, Vec<(PathBuf, PathBuf)>> {
    LIVE.lock().unwrap_or_else(|held| held.into_inner())
}

#[cfg(unix)]
pub(super) fn same(path: &Path, file: &File) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::metadata(path), file.metadata()) {
        (Ok(named), Ok(held)) => named.dev() == held.dev() && named.ino() == held.ino(),
        _ => false,
    }
}

#[cfg(not(unix))]
pub(super) fn same(path: &Path, _: &File) -> bool {
    path.exists()
}

#[cfg(test)]
mod tests {
    use super::contains;

    #[test]
    fn staged() {
        let root = tempfile::tempdir().expect("root");
        let staged = root.path().join("guard-a1b2");
        std::fs::create_dir(&staged).expect("staged");
        assert!(contains(root.path(), &staged));
        assert!(contains(root.path(), &root.path().join("guard-gone")));
    }

    #[test]
    fn foreign() {
        let root = tempfile::tempdir().expect("root");
        let other = tempfile::tempdir().expect("other");
        assert!(!contains(root.path(), &other.path().join("guard-a1b2")));
        assert!(!contains(root.path(), &root.path().join("linked")));
        assert!(!contains(
            root.path(),
            &root.path().join("nested/guard-a1b2")
        ));
        assert!(!contains(root.path(), root.path()));
    }
}
