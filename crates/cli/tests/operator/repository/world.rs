use std::path::Path;
use std::process::Command;

pub fn govern(root: &Path) {
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
    declare(root);
}

pub fn declare(root: &Path) {
    let hooks = root.join(".git/hooks");
    for name in ["pre-commit", "commit-msg"] {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/git/hooks")
            .join(name);
        let target = hooks.join(name);
        std::fs::copy(source, &target).expect("fixture should carry the guard hooks");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))
                .expect("fixture hook should be executable");
        }
    }
    let held = "[[layout.seat]]\npath = \"charts/*\"\nkind = \"retired\"\n\n\
                [[layout.seat]]\npath = \"skills/*\"\nkind = \"retired\"\n\n\
                [[layout.file]]\nname = [\"AGENTS.md\"]\nkind = \"retired\"\n";
    std::fs::write(root.join("plumb.toml"), held).expect("fixture should declare itself");
}

pub fn run(root: &Path) -> String {
    let home = super::support::home();
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .env_remove("PLUMB_RELEASE_VERSION")
        .env("PLUMB_HOME", home.path())
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn boundary() {
    let repo = super::precommit::Repo::new();
    let output = repo.plumb("src");
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("report");
    assert_eq!(report["schema"], "plumb.precommit/v1");
    assert_eq!(report["base"], repo.base);
    assert_eq!(report["head"], repo.head);
    assert_eq!(
        report["changed"],
        serde_json::json!(["README.md", "src/lib.rs"])
    );
    assert_eq!(report["outside"], serde_json::json!(["README.md"]));
    assert_eq!(report["ok"], false);
}
