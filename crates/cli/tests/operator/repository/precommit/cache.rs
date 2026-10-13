use super::{Repo, support};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) fn fixture() -> tempfile::TempDir {
    let held = tempfile::tempdir().expect("repository");
    let root = held.path();
    Repo::git(root, &["init", "-q", "-b", "topic"]);
    Repo::git(root, &["config", "user.name", "Plumb Test"]);
    Repo::git(root, &["config", "user.email", "plumb@example.invalid"]);
    Repo::git(
        root,
        &[
            "remote",
            "add",
            "origin",
            "https://git.example.invalid/Example/probe.git",
        ],
    );
    std::fs::create_dir(root.join("src")).expect("source");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("manifest");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    42\n}\n",
    )
    .expect("source");
    std::fs::write(
        root.join("plumb.toml"),
        "[workflow.hash.guard]\nrust = [\"Cargo.toml\", \"Cargo.lock\", \"src\"]\n",
    )
    .expect("governance");
    let output = Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(root)
        .output()
        .expect("lock");
    assert!(output.status.success());
    Repo::git(
        root,
        &["add", "Cargo.toml", "Cargo.lock", "src", "plumb.toml"],
    );
    held
}

pub(crate) fn run(root: &Path, home: &Path) -> Output {
    super::support::plumb()
        .args(["guard", ".", "--json"])
        .current_dir(root)
        .env("PLUMB_HOME", home)
        .env("PLUMB_GUARD_CACHE_RESERVE", "0")
        .env("CARGO_TARGET_DIR", root.join("unowned"))
        .output()
        .expect("guard")
}

pub(crate) fn seats(home: &Path) -> Vec<PathBuf> {
    #[cfg(windows)]
    let root = home.join("c");
    #[cfg(not(windows))]
    let root = home.join("cache/guard/cargo");
    std::fs::read_dir(root)
        .expect("cache")
        .map(|entry| entry.expect("entry").path())
        .collect()
}

pub(crate) fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn retained() {
    let fixture = fixture();
    let root = fixture.path();
    let home = support::home();
    success(&run(root, home.path()));
    let held = seats(home.path());
    assert_eq!(held.len(), 1);
    assert!(held[0].join("debug/deps").is_dir());
    assert!(!root.join("target").exists());
    assert!(!root.join("unowned").exists());
    let retained = held[0].join("retained");
    std::fs::write(&retained, "material, not proof").expect("retained cache");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    43\n}\n",
    )
    .expect("changed source");
    Repo::git(root, &["add", "src/lib.rs"]);
    let changed = run(root, home.path());
    success(&changed);
    assert!(String::from_utf8_lossy(&changed.stderr).contains("guard guard/rust"));
    assert_eq!(seats(home.path()), held);
    assert!(retained.is_file());
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    false\n}\n",
    )
    .expect("invalid source");
    Repo::git(root, &["add", "src/lib.rs"]);
    let refused = run(root, home.path());
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("mismatched types"));
    assert!(retained.is_file());
}

#[test]
fn isolated() {
    let first = fixture();
    let second = fixture();
    let home = support::home();
    success(&run(first.path(), home.path()));
    std::fs::write(
        second.path().join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    44\n}\n",
    )
    .expect("different input");
    Repo::git(second.path(), &["add", "src/lib.rs"]);
    success(&run(second.path(), home.path()));
    assert_eq!(seats(home.path()).len(), 2);
}

fn compiled(root: &Path, home: &Path, value: &str, path: &std::ffi::OsStr) -> Output {
    super::support::plumb()
        .args(["guard", ".", "--json"])
        .current_dir(root)
        .env("PLUMB_HOME", home)
        .env("PLUMB_GUARD_CACHE_RESERVE", "0")
        .env("PLUMB_GUARD_COMPILER_CACHE", value)
        .env("PATH", path)
        .output()
        .expect("guard")
}

#[cfg(unix)]
fn without(program: &str, seat: &Path) -> std::ffi::OsString {
    for entry in std::env::split_paths(&std::env::var_os("PATH").expect("PATH")) {
        let Ok(read) = std::fs::read_dir(&entry) else {
            continue;
        };
        for tool in read.flatten() {
            let link = seat.join(tool.file_name());
            if tool.file_name() != program && !link.exists() {
                std::os::unix::fs::symlink(tool.path(), link).expect("tool link");
            }
        }
    }
    seat.as_os_str().to_owned()
}

#[test]
#[cfg(unix)]
fn compiler() {
    let fixture = fixture();
    let home = support::home();
    let path = std::env::var_os("PATH").expect("PATH");
    let refused = compiled(fixture.path(), home.path(), "ccache", &path);
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr)
            .contains("PLUMB_GUARD_COMPILER_CACHE must be sccache"),
        "{}{}",
        String::from_utf8_lossy(&refused.stdout),
        String::from_utf8_lossy(&refused.stderr)
    );
    let seat = tempfile::tempdir().expect("tools");
    let absent = compiled(
        fixture.path(),
        home.path(),
        "sccache",
        &without("sccache", seat.path()),
    );
    assert!(!absent.status.success());
    assert!(
        String::from_utf8_lossy(&absent.stderr).contains("cannot resolve tool sccache"),
        "{}{}",
        String::from_utf8_lossy(&absent.stdout),
        String::from_utf8_lossy(&absent.stderr)
    );
    assert!(!home.path().join("cache/compiler").exists());
}

#[test]
#[ignore = "explicit integration with the canonical image's exact sccache binary"]
fn routed() {
    let fixture = fixture();
    let home = support::home();
    let path = std::env::var_os("PATH").expect("PATH");
    success(&compiled(fixture.path(), home.path(), "sccache", &path));
    let held = seats(home.path());
    assert_eq!(held.len(), 1);
    assert!(
        std::fs::read_dir(held[0].join("debug/incremental"))
            .map_or(true, |mut entries| entries.next().is_none())
    );
    assert!(
        std::fs::read_dir(home.path().join("cache/compiler"))
            .expect("compiler cache")
            .next()
            .is_some()
    );
}
