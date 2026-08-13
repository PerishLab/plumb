use super::fixture::SPEC;
use std::path::Path;
use std::process::{Command, Output};

const GIT: &[&str] = &[
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
];

fn run(command: &mut Command) -> Output {
    let output = command.output().expect("command should run");
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn git(root: &Path, args: &[&str]) -> String {
    let mut command = Command::new("git");
    command.current_dir(root).args(args);
    isolate(&mut command);
    let output = run(&mut command);
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn plumb(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command.env("PLUMB_RELEASE_ROOT", root);
    isolate(&mut command);
    command
}

fn isolate(command: &mut Command) {
    for name in GIT {
        command.env_remove(name);
    }
}

#[test]
fn topology() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    std::fs::write(root.join("plumb.toml"), SPEC).expect("release manifest");
    git(root, &["init", "-b", "main"]);
    git(root, &["config", "user.name", "Plumb Test"]);
    git(root, &["config", "user.email", "plumb@example.test"]);
    git(root, &["add", "plumb.toml"]);
    git(root, &["commit", "-m", "main"]);
    let main = git(root, &["rev-parse", "HEAD"]);

    git(root, &["tag", "v1.2.0-beta.1"]);
    run(plumb(root)
        .args(["release", "source"])
        .env("PLUMB_RELEASE_CHANNEL", "beta")
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.1")
        .env("PLUMB_RELEASE_COMMIT", &main)
        .env("PLUMB_RELEASE_SOURCE", "refs/tags/v1.2.0-beta.1"));

    let branched = plumb(root)
        .args(["release", "source"])
        .env("PLUMB_RELEASE_CHANNEL", "beta")
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.1")
        .env("PLUMB_RELEASE_COMMIT", &main)
        .env("PLUMB_RELEASE_SOURCE", "refs/heads/main")
        .output()
        .expect("plumb should run");
    assert!(!branched.status.success());
    assert!(
        String::from_utf8_lossy(&branched.stderr).contains("refs/tags/v1.2.0-beta.1"),
        "{}",
        String::from_utf8_lossy(&branched.stderr)
    );

    run(plumb(root)
        .args(["release", "channel"])
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.1"));

    let wrong = plumb(root)
        .args(["release", "source"])
        .env("PLUMB_RELEASE_CHANNEL", "stable")
        .env("PLUMB_RELEASE_VERSION", "v1.2.0")
        .env("PLUMB_RELEASE_COMMIT", &main)
        .env("PLUMB_RELEASE_SOURCE", "refs/heads/main")
        .output()
        .expect("plumb should run");
    assert!(!wrong.status.success());
    assert!(String::from_utf8_lossy(&wrong.stderr).contains("refs/heads/release/v1.2.0"));

    git(root, &["checkout", "-b", "release/v1.2.0"]);
    git(root, &["commit", "--allow-empty", "-m", "stable"]);
    let stable = git(root, &["rev-parse", "HEAD"]);
    run(plumb(root)
        .args(["release", "source"])
        .env("PLUMB_RELEASE_CHANNEL", "stable")
        .env("PLUMB_RELEASE_VERSION", "v1.2.0")
        .env("PLUMB_RELEASE_COMMIT", &stable)
        .env("PLUMB_RELEASE_SOURCE", "refs/heads/release/v1.2.0"));

    git(root, &["checkout", "main"]);
    let unsettled = plumb(root)
        .args(["release", "packport"])
        .env("PLUMB_RELEASE_VERSION", "v1.2.0")
        .env("PLUMB_RELEASE_COMMIT", &stable)
        .env("PLUMB_RELEASE_BASE", "main")
        .output()
        .expect("plumb should run");
    assert!(!unsettled.status.success());
    assert!(String::from_utf8_lossy(&unsettled.stderr).contains("is not an ancestor"));

    git(
        root,
        &[
            "merge",
            "--no-ff",
            "release/v1.2.0",
            "-m",
            "Packport v1.2.0",
        ],
    );
    run(plumb(root)
        .args(["release", "packport"])
        .env("PLUMB_RELEASE_VERSION", "v1.2.0")
        .env("PLUMB_RELEASE_COMMIT", &stable)
        .env("PLUMB_RELEASE_BASE", "main"));
}
