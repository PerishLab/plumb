use std::path::Path;
use std::process::{Command, Output};

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn fixture() -> tempfile::TempDir {
    let root = super::super::fixture();
    let declaration = std::fs::read_to_string(root.path().join("plumb.toml")).unwrap();
    std::fs::write(
        root.path().join("plumb.toml"),
        format!("{declaration}\n[[layout.file]]\nname=['package.json','pnpm-lock.yaml']\n"),
    )
    .unwrap();
    std::fs::write(
        root.path().join("package.json"),
        r#"{"dependencies":{"@perishlab/design":"0"}}"#,
    )
    .unwrap();
    std::fs::write(root.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\nimporters:\n  .:\n    dependencies:\n      '@perishlab/design':\n        specifier: '0'\n        version: 0.3.14(svelte@5.0.0)\npackages:\n  '@perishlab/design@0.3.14': {}\n").unwrap();
    git(root.path(), &["add", "."]);
    root
}

fn observed(root: &Path, latest: &str) -> serde_json::Value {
    serde_json::json!({"schema":1,"resolution":{"context":"ci-latest","tree":git(root, &["write-tree"]),"packages":[{"ecosystem":"npm","name":"@perishlab/design","version":latest}]}})
}

fn run(root: &Path, observation: &serde_json::Value) -> Output {
    let home = crate::support::home();
    crate::support::plumb()
        .args(["doctor", root.to_str().unwrap()])
        .env_remove("PLUMB_RELEASE_VERSION")
        .env_remove("WHARF_PACKAGES_TOKEN")
        .env(
            "NPM_CONFIG_USERCONFIG",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("PLUMB_PACKAGES_OBSERVATION", observation.to_string())
        .env("PLUMB_HOME", home.path())
        .output()
        .unwrap()
}

#[test]
fn currency() {
    let root = fixture();
    let current = run(root.path(), &observed(root.path(), "0.3.14"));
    let said = String::from_utf8_lossy(&current.stdout);
    assert!(current.status.success(), "{said}");
    assert!(
        said.contains("npm @perishlab/design 0 -> 0.3.14 (latest 0.3.14)"),
        "{said}"
    );
    let stale = run(root.path(), &observed(root.path(), "0.3.15"));
    let said = String::from_utf8_lossy(&stale.stdout);
    assert!(!stale.status.success(), "{said}");
    assert!(
        said.contains("0.3.14") && said.contains("0.3.15") && said.contains("out of true"),
        "{said}"
    );
}

#[test]
fn blind() {
    let root = fixture();
    let mut observation = observed(root.path(), "0.3.14");
    observation["schema"] = 99.into();
    let output = run(root.path(), &observation);
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success(), "{said}");
    assert!(
        said.contains("package observation schema or context differs"),
        "{said}"
    );
    assert!(!said.contains("true to the skeleton"), "{said}");
    let observation = observed(root.path(), "0.3.14");
    std::fs::write(root.path().join("changed"), "different tree").unwrap();
    git(root.path(), &["add", "changed"]);
    let output = run(root.path(), &observation);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("package observation tree differs"));
}

#[test]
fn datum() {
    let root = fixture();
    git(
        root.path(),
        &["symbolic-ref", "HEAD", "refs/heads/release/v1.2.0"],
    );
    let seat = root.path().join(".plumb/releases/v1.2.0");
    std::fs::create_dir_all(&seat).unwrap();
    std::fs::write(seat.join("datum.toml"), "schema=1\nversion='v1.2.0'\n[[answer]]\necosystem='npm'\nname='@perishlab/design'\nlatest='0.3.14'\n").unwrap();
    let output = run(root.path(), &observed(root.path(), "0.3.15"));
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{said}");
    assert!(said.contains("latest 0.3.14"), "{said}");
    assert!(!said.contains("latest 0.3.15"), "{said}");
}
