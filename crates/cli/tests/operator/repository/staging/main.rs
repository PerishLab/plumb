mod owner;
#[cfg(target_os = "linux")]
mod parallel;
mod source;

use super::precommit::{Repo, atoms, cache};
use super::support;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime};

struct Gated {
    repository: tempfile::TempDir,
    signals: tempfile::TempDir,
}

impl Gated {
    fn new() -> Self {
        let repository = cache::fixture();
        let signals = tempfile::tempdir().expect("signals");
        let root = repository.path();
        std::fs::write(signals.path().join("gate"), "").expect("gate");
        std::fs::write(
            root.join("build.rs"),
            format!(
                "fn main() {{\n    std::fs::write({:?}, \"\").unwrap();\n    while std::path::Path::new({:?}).exists() {{\n        std::thread::sleep(std::time::Duration::from_millis(50));\n    }}\n}}\n",
                signals.path().join("started"),
                signals.path().join("gate"),
            ),
        )
        .expect("build script");
        std::fs::write(
            root.join("plumb.toml"),
            "[workflow.hash.guard]\nrust = [\"Cargo.toml\", \"Cargo.lock\", \"src\", \"build.rs\"]\n",
        )
        .expect("governance");
        Repo::git(root, &["add", "build.rs", "plumb.toml"]);
        Self {
            repository,
            signals,
        }
    }

    fn root(&self) -> &Path {
        self.repository.path()
    }

    fn spawn(&self, home: &Path, group: bool) -> Child {
        let mut command = support::plumb();
        command
            .args(["guard", ".", "--json"])
            .current_dir(self.root())
            .env("PLUMB_HOME", home)
            .env("PLUMB_GUARD_CACHE_RESERVE", "0")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if group {
            std::os::unix::process::CommandExt::process_group(&mut command, 0);
        }
        let mut guard = command.spawn().expect("guard");
        for _ in 0..1200 {
            if self.signals.path().join("started").exists() {
                return guard;
            }
            assert!(
                guard.try_wait().expect("guard state").is_none(),
                "guard finished before its check started"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("guard check never started");
    }

    fn open(&self) {
        let _ = std::fs::remove_file(self.signals.path().join("gate"));
    }
}

fn staged(home: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(home.join("tmp")) else {
        return Vec::new();
    };
    let mut held = entries
        .map(|entry| entry.expect("entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("guard-"))
        })
        .collect::<Vec<_>>();
    held.sort();
    held
}

fn registered(root: &Path) -> Vec<PathBuf> {
    String::from_utf8(Repo::git(root, &["worktree", "list", "--porcelain"]).stdout)
        .expect("utf8")
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(PathBuf::from)
        .filter(|path| path.canonicalize().ok() != root.canonicalize().ok())
        .collect()
}

fn signal(guard: &Child, name: &str, group: bool) {
    let target = if group {
        format!("-{}", guard.id())
    } else {
        guard.id().to_string()
    };
    let status = Command::new("kill")
        .args([name, "--", &target])
        .status()
        .expect("kill");
    assert!(status.success());
}

fn retained(home: &Path, root: &Path) {
    let trees = registered(root);
    assert_eq!(trees.len(), 1, "one retained registration");
    assert!(trees[0].join("src/lib.rs").is_file());
    assert!(staged(home).contains(&trees[0].with_extension("lock")));
    assert!(
        Repo::git(
            &trees[0],
            &["status", "--porcelain", "--untracked-files=no"]
        )
        .stdout
        .is_empty()
    );
}

#[test]
fn killed() {
    let gated = Gated::new();
    let home = support::home();
    let mut guard = gated.spawn(home.path(), true);
    signal(&guard, "-KILL", true);
    guard.wait().expect("killed guard");
    gated.open();
    let leaked = staged(home.path())
        .into_iter()
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    assert_eq!(leaked.len(), 1, "a killed guard leaves its tree");
    assert_eq!(registered(gated.root()).len(), 1, "and its registration");
    cache::success(&cache::run(gated.root(), home.path()));
    retained(home.path(), gated.root());
}

#[test]
fn legacy() {
    let fixture = cache::fixture();
    let root = fixture.path();
    let home = support::home();
    Repo::git(root, &["commit", "-q", "-m", "base"]);
    let base = home.path().join("tmp");
    std::fs::create_dir_all(&base).expect("staging root");
    for name in ["guard-legacy", "guard-young"] {
        let tree = base.join(name);
        Repo::git(
            root,
            &[
                "worktree",
                "add",
                "-q",
                "--detach",
                tree.to_str().expect("path"),
            ],
        );
    }
    std::fs::File::open(base.join("guard-legacy"))
        .and_then(|tree| tree.set_modified(SystemTime::now() - Duration::from_secs(7200)))
        .expect("aged tree");
    cache::success(&cache::run(root, home.path()));
    assert!(base.join("guard-young").is_dir());
    assert!(!base.join("guard-legacy").exists());
    assert_eq!(
        registered(root).len(),
        2,
        "the young tree and retained source stay registered"
    );
}

#[test]
fn live() {
    let gated = Gated::new();
    let home = support::home();
    let guard = gated.spawn(home.path(), false);
    let held = staged(home.path());
    assert!(!held.is_empty(), "the gated guard holds a tree");
    let other = cache::fixture();
    cache::success(&cache::run(other.path(), home.path()));
    for path in &held {
        assert!(path.exists(), "a live tree was reaped: {}", path.display());
    }
    assert_eq!(
        registered(gated.root()).len(),
        1,
        "the live tree stays registered"
    );
    gated.open();
    cache::success(&guard.wait_with_output().expect("gated guard"));
    retained(home.path(), gated.root());
    retained(home.path(), other.path());
}

#[test]
fn terminated() {
    let gated = Gated::new();
    let home = support::home();
    std::fs::write(
        home.path().join("plumb.toml"),
        "[locus]\nenabled = true\nendpoint = 'http://127.0.0.1:0'\n",
    )
    .expect("observation");
    let mut guard = gated.spawn(home.path(), false);
    signal(&guard, "-TERM", false);
    let exited = (0..600).any(|_| {
        std::thread::sleep(Duration::from_millis(100));
        guard.try_wait().expect("guard state").is_some()
    });
    gated.open();
    let output = guard.wait_with_output().expect("terminated guard");
    assert!(exited, "a terminated guard kept running");
    assert_eq!(
        output.status.code(),
        Some(143),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    retained(home.path(), gated.root());
    cache::success(&cache::run(gated.root(), home.path()));
    retained(home.path(), gated.root());
    let atoms = atoms(home.path());
    let edges = |edge: &str| {
        atoms
            .iter()
            .filter(|atom| atom["source"]["edge"] == edge)
            .count()
    };
    assert!(
        atoms
            .iter()
            .any(|atom| atom["source"]["function"] == "execute"
                && atom["source"]["edge"] == "return"),
        "the interrupted check span closes: {atoms:?}"
    );
    assert_eq!(
        edges("enter"),
        edges("return"),
        "every span closes: {atoms:?}"
    );
}
