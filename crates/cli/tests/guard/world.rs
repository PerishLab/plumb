use std::process::Command;

pub(crate) fn seat() -> std::path::PathBuf {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("git should run");
    assert!(
        output.status.success(),
        "the guard runs inside its repository"
    );
    std::path::PathBuf::from(String::from_utf8_lossy(&output.stdout).trim())
}

pub(crate) fn govern(root: &std::path::Path) {
    let status = Command::new("git")
        .args([
            "-C",
            root.to_str().expect("path should be utf8"),
            "init",
            "-q",
        ])
        .status()
        .expect("git should run");
    assert!(status.success(), "fixture should become a repository");
    super::support::hooks(root);
}

pub(crate) fn fixture() -> tempfile::TempDir {
    let seat = tempfile::tempdir().expect("fixture");
    govern(seat.path());
    declare(seat.path());
    seat
}

pub(crate) fn declare(root: &std::path::Path) {
    let held = "[[layout.seat]]\npath = \"charts/*\"\nkind = \"retired\"\n\n\
                [[layout.seat]]\npath = \"skills/*\"\nkind = \"retired\"\n\n\
                [[layout.file]]\nname = [\"AGENTS.md\"]\nkind = \"retired\"\n";
    std::fs::write(root.join("plumb.toml"), held).expect("fixture should declare itself");
}

pub(crate) fn run(args: &[&str]) -> String {
    let home = super::support::home();
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(args)
        .env_remove("PLUMB_RELEASE_VERSION")
        .env("PLUMB_HOME", home.path())
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}
