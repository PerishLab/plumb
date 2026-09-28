use super::{Repo, support};
use std::path::Path;
use std::process::Command;

const PROBE: &str = r#"use std::process::Command;

#[test]
fn unhooked() {
    assert!(std::env::vars().all(|(key, _)| !key.starts_with("GIT_")));
    let root = std::env::temp_dir().join(format!("plumb-unhooked-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "plumb.leak", "written"],
        &["-c", "user.name=Probe", "-c", "user.email=probe@example.invalid", "commit", "-q", "--allow-empty", "-m", "leak"],
        &["tag", "leaked"],
    ] {
        assert!(Command::new("git").args(args).current_dir(&root).status().unwrap().success());
    }
    std::fs::remove_dir_all(&root).unwrap();
}
"#;

fn seed(root: &Path) {
    Repo::git(root, &["init", "-q", "-b", "main"]);
    std::fs::create_dir_all(root.join("src")).expect("source");
    std::fs::create_dir_all(root.join("tests")).expect("tests");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("manifest");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    42\n}\n",
    )
    .expect("lib");
    std::fs::write(root.join("tests/probe.rs"), PROBE).expect("probe");
    std::fs::write(
        root.join("plumb.toml"),
        "[workflow.hash.guard]\ntest = [\"Cargo.toml\", \"Cargo.lock\", \"src\", \"tests\"]\n",
    )
    .expect("governance");
    let lock = Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(root)
        .output()
        .expect("lock");
    assert!(lock.status.success());
    Repo::git(root, &["add", "-A"]);
    Repo::git(
        root,
        &[
            "-c",
            "user.name=Plumb Test",
            "-c",
            "user.email=plumb@example.invalid",
            "commit",
            "-q",
            "--no-verify",
            "-m",
            "seed",
        ],
    );
}

fn state(repository: &Path) -> (Vec<u8>, String) {
    let config = std::fs::read(repository.join("config")).expect("config");
    let refs = Repo::git(
        repository,
        &["for-each-ref", "--format=%(refname) %(objectname)"],
    );
    let refs = String::from_utf8(refs.stdout).expect("utf8");
    let refs = refs
        .lines()
        .filter(|line| !line.starts_with("refs/heads/topic "))
        .collect::<Vec<_>>()
        .join("\n");
    (config, refs)
}

#[test]
#[cfg(unix)]
fn linked() {
    let fixture = tempfile::tempdir().expect("fixture");
    let seat = fixture.path().join("seed");
    let repository = fixture.path().join("repository.git");
    let worktree = fixture.path().join("linked");
    std::fs::create_dir(&seat).expect("seed");
    seed(&seat);
    let clone = Command::new("git")
        .args(["clone", "-q", "--bare"])
        .arg(&seat)
        .arg(&repository)
        .output()
        .expect("clone");
    assert!(clone.status.success());
    let add = Command::new("git")
        .arg("-C")
        .arg(&repository)
        .args(["worktree", "add", "-q", "-b", "topic"])
        .arg(&worktree)
        .arg("main")
        .output()
        .expect("worktree");
    assert!(add.status.success());
    let home = support::home();
    let install = support::plumb()
        .args(["configuration", "install"])
        .arg(&worktree)
        .env("PLUMB_HOME", home.path())
        .output()
        .expect("install");
    assert!(install.status.success());
    assert!(repository.join("hooks/pre-commit").is_file());

    let before = state(&repository);
    std::fs::write(
        worktree.join("src/lib.rs"),
        "pub fn answer() -> u8 {\n    43\n}\n",
    )
    .expect("change");
    Repo::git(&worktree, &["add", "src/lib.rs"]);
    let binary = Path::new(env!("CARGO_BIN_EXE_plumb"))
        .parent()
        .expect("bin");
    let path = std::env::join_paths(std::iter::once(binary.to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .expect("path");
    let commit = support::outside("git")
        .arg("-C")
        .arg(&worktree)
        .args([
            "-c",
            "user.name=Plumb Test",
            "-c",
            "user.email=plumb@example.invalid",
            "commit",
            "-q",
            "-m",
            "candidate",
        ])
        .env("PATH", path)
        .env("PLUMB_HOME", home.path())
        .output()
        .expect("commit");
    assert!(
        commit.status.success(),
        "{}{}",
        String::from_utf8_lossy(&commit.stdout),
        String::from_utf8_lossy(&commit.stderr)
    );
    assert!(String::from_utf8_lossy(&commit.stderr).contains("guard guard/test"));
    plumb::guard::commit(&worktree, "HEAD").expect("committed proof");
    assert_eq!(state(&repository), before);
    let tags = Repo::git(&repository, &["tag", "-l"]);
    assert!(tags.stdout.is_empty());
}
