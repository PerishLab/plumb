use std::path::Path;
use std::process::{Command, Output};

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn put(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn run(root: &Path, home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["lock", "resolve", root.to_str().unwrap(), "--json"])
        .env("PLUMB_HOME", home)
        .env_remove("SCCACHE_DIR")
        .env_remove("RUSTC_WRAPPER")
        .output()
        .unwrap()
}

const SOURCE: &str = r#"use std::fs::File as Handle;
type Gate = Handle;
pub fn direct(file: &Handle) {
    let _ = file.lock();
    let _ = file.lock_shared();
    let _ = file.try_lock();
    let _ = file.try_lock_shared();
    let _ = file.unlock();
}
pub fn alias(file: &Gate) { let _ = file.try_lock(); }
macro_rules! scoped { ($file:expr) => { $file.try_lock() }; }
pub fn local(file: &Handle) { let _ = scoped!(file); }
pub fn external(file: &Handle) { let _ = carrier::protect!(file); }
pub fn pointer(file: &Handle) { let action = Handle::try_lock; let _ = action(file); }
#[allow(clippy::disallowed_methods)]
pub fn suppressed(file: &Handle) { let _ = file.try_lock(); }
#[cfg(feature = "extra")]
pub fn gated(file: &Handle) { let _ = file.try_lock(); }
pub fn data(value: &std::sync::Mutex<Vec<u8>>) { drop(value.lock()); }
pub fn unit(value: &std::sync::Mutex<()>) { drop(value.lock()); }
pub fn stdio() { drop(std::io::stdout().lock()); }
"#;

#[test]
fn compiler() {
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let root = root.path();
    put(
        root,
        "Cargo.toml",
        "[workspace]\nmembers=[\"app\",\"carrier\"]\nresolver=\"2\"\n",
    );
    put(root, "plumb.toml", "[identity]\nns=\"probe\"\n");
    put(
        root,
        "Cargo.lock",
        "version=4\n[[package]]\nname=\"probe\"\nversion=\"0.0.0\"\ndependencies=[\"carrier\"]\n[[package]]\nname=\"carrier\"\nversion=\"0.0.0\"\n",
    );
    put(
        root,
        "app/Cargo.toml",
        "[package]\nname=\"probe\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[features]\nextra=[]\n[dependencies]\ncarrier={path=\"../carrier\"}\n",
    );
    put(
        root,
        "carrier/Cargo.toml",
        "[package]\nname=\"carrier\"\nversion=\"0.0.0\"\nedition=\"2024\"\n",
    );
    put(
        root,
        "carrier/src/lib.rs",
        "#[macro_export]\nmacro_rules! protect { ($file:expr) => { $file.try_lock() }; }\n",
    );
    put(root, "app/src/lib.rs", SOURCE);
    git(root, &["init", "-q"]);
    git(root, &["add", "."]);
    let output = run(root, home.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema"], "plumb.lock-resolution/v1");
    assert_eq!(report["guard"], false);
    let findings = report["messages"]["findings"].as_array().unwrap();
    for operation in [
        "lock",
        "lock_shared",
        "try_lock",
        "try_lock_shared",
        "unlock",
    ] {
        assert!(
            findings
                .iter()
                .any(|finding| finding["operation"] == format!("std::fs::File::{operation}")),
            "{operation}"
        );
    }
    for line in [10, 11, 14, 16] {
        assert!(
            findings.iter().any(|finding| finding["spans"]
                .as_array()
                .unwrap()
                .iter()
                .any(|span| span["line_start"] == line)),
            "missing alias/macro/pointer/allow at line {line}"
        );
    }
    assert!(!findings.iter().any(|finding| {
        finding["spans"]
            .as_array()
            .unwrap()
            .iter()
            .any(|span| span["line_start"] == 18)
    }));
    assert!(findings.iter().any(|finding| {
        finding["spans"]
            .as_array()
            .unwrap()
            .iter()
            .any(|span| span["expansion"].is_object())
    }));
    assert!(
        findings.iter().any(
            |finding| finding["spans"]
                .as_array()
                .unwrap()
                .iter()
                .any(|span| span["file_name"]
                    .as_str()
                    .is_some_and(|name| Path::new(name).ends_with("carrier/src/lib.rs"))
                    && span["expansion"]["span"]["line_start"] == 13)
        )
    );
    assert!(report["tools"]["cargo"]["digest"].as_str().is_some());
    assert!(report["execution"].is_string());
    let cached = run(root, home.path());
    assert!(
        cached.status.success(),
        "{}",
        String::from_utf8_lossy(&cached.stderr)
    );
    let cached: serde_json::Value = serde_json::from_slice(&cached.stdout).unwrap();
    assert_eq!(
        cached["messages"]["findings"],
        report["messages"]["findings"]
    );
    put(root, "app/src/lib.rs", "pub fn unstaged() {}\n");
    let staged = run(root, home.path());
    assert!(
        staged.status.success(),
        "{}",
        String::from_utf8_lossy(&staged.stderr)
    );
    let staged: serde_json::Value = serde_json::from_slice(&staged.stdout).unwrap();
    assert_eq!(staged["source"], report["source"]);
    assert_eq!(
        staged["messages"]["findings"],
        report["messages"]["findings"]
    );
    git(root, &["add", "app/src/lib.rs"]);
    let clean = run(root, home.path());
    assert!(
        clean.status.success(),
        "{}",
        String::from_utf8_lossy(&clean.stderr)
    );
    let clean: serde_json::Value = serde_json::from_slice(&clean.stdout).unwrap();
    assert!(clean["messages"]["findings"].as_array().unwrap().is_empty());
    put(
        root,
        "app/src/lib.rs",
        "pub fn data(value: &std::sync::Mutex<()>) { drop(value.lock()); }\n",
    );
    put(
        root,
        "clippy.toml",
        "disallowed-methods=[{path=\"std::mem::drop\",reason=\"plumb.lock/std::fs::File::lock\"}]\n",
    );
    git(root, &["add", "."]);
    let mismatch = run(root, home.path());
    assert_eq!(
        mismatch.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&mismatch.stderr)
    );
    assert!(mismatch.stdout.is_empty());
    assert!(String::from_utf8_lossy(&mismatch.stderr).contains("managed lock marker differs"));
    std::fs::remove_file(root.join("clippy.toml")).unwrap();
    git(root, &["add", "clippy.toml"]);
    put(
        root,
        "plumb.toml",
        "[identity]\nns=\"probe\"\n[workflow.hash.guard]\nrust=[\"suite://cargo\"]\ntest=[\"suite://cargo\"]\n[[workflow.tool.guard.test]]\nprogram=\"missing-lock-test-prerequisite\"\n",
    );
    git(root, &["add", "plumb.toml"]);
    let selected = run(root, home.path());
    assert!(
        selected.status.success(),
        "unselected action prerequisite blocked Rust collection: {}",
        String::from_utf8_lossy(&selected.stderr)
    );
    put(root, "app/src/lib.rs", "pub fn broken(\n");
    git(root, &["add", "app/src/lib.rs"]);
    let failed = run(root, home.path());
    assert_eq!(failed.status.code(), Some(2));
    assert!(failed.stdout.is_empty());
}
