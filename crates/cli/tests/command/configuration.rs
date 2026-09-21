use std::process::Command;

#[path = "../support/mod.rs"]
pub(super) mod support;

#[test]
#[cfg(unix)]
fn install() {
    let home = support::home();
    let repo = tempfile::tempdir().expect("repository");
    let status = Command::new("git")
        .args(["-C", repo.path().to_str().expect("repo"), "init", "-q"])
        .status()
        .expect("git");
    assert!(status.success());
    let install = || {
        Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args([
                "configuration",
                "install",
                repo.path().to_str().expect("repo"),
            ])
            .env("PLUMB_HOME", home.path())
            .output()
            .expect("install")
    };
    let bare = install();
    assert!(bare.status.success());
    assert!(String::from_utf8_lossy(&bare.stdout).contains("no hooks projected"));
    assert!(!repo.path().join(".git/hooks/pre-commit").exists());

    std::fs::write(repo.path().join("plumb.toml"), "[layout]\n").expect("manifest");
    let output = install();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in ["pre-commit", "commit-msg"] {
        let carried = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets/git/hooks")
                .join(name),
        )
        .expect("carried hook");
        assert_eq!(
            std::fs::read_to_string(repo.path().join(".git/hooks").join(name)).expect("hook"),
            carried
        );
    }
    assert!(String::from_utf8_lossy(&output.stdout).contains("projected Plumb guard hooks"));
    assert!(!home.path().join("configurations").exists());
}
