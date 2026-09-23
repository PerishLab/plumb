use std::process::Command;

pub(crate) fn seat() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crate should sit two below the repo")
        .to_path_buf()
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
    let hooks = root.join(".git/hooks");
    for name in ["pre-commit", "commit-msg"] {
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/git/hooks")
            .join(name);
        let target = hooks.join(name);
        std::fs::copy(source, &target).expect("fixture should carry depot-projected guard hooks");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))
                .expect("fixture hook should be executable");
        }
    }
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
