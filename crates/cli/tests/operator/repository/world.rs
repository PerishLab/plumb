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
}

pub fn run(root: &Path) -> String {
    let home = super::support::depot(&[]);
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .env_remove("PLUMB_RELEASE_VERSION")
        .env("PLUMB_HOME", home.path())
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn profile(product: &str, manifest: &str, ectropy: &str) -> String {
    format!(
        "schema = \"plumb.product-profile/v1\"\n\n[product]\nname = \"{product}\"\nauthority = \"https://releases.{product}.perish.uk\"\nderivatives = [\"skill\"]\n\n[governance]\nmanifest = '''\n{manifest}'''\nectropy = '''\n{ectropy}'''\n"
    )
}

#[cfg(unix)]
pub fn hooks(root: &Path) {
    use std::os::unix::fs::PermissionsExt as _;

    for name in ["pre-commit", "commit-msg"] {
        let hook = root.join(".git/hooks").join(name);
        std::fs::write(&hook, "#!/bin/sh\nexit 0\n").expect("hook");
        let mut mode = std::fs::metadata(&hook)
            .expect("hook metadata")
            .permissions();
        mode.set_mode(0o755);
        std::fs::set_permissions(&hook, mode).expect("hook mode");
    }
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
