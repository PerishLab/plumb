use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn seat(name: &str, version: Option<&str>) -> PathBuf {
    let path = std::env::temp_dir().join(format!("plumb-changelog-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("seat");
    if let Some(version) = version {
        fs::write(
            path.join("Cargo.toml"),
            format!("[workspace.package]\nversion = \"{version}\"\n"),
        )
        .expect("cargo");
    }
    path
}

fn marked(workspace: &Path, extra: &[&str]) -> (String, bool) {
    let mut args = vec!["changelog", workspace.to_str().expect("path")];
    args.extend_from_slice(extra);
    let out = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(&args)
        .output()
        .expect("run");
    let shown = String::from_utf8_lossy(&out.stdout).to_string();
    (shown, out.status.success())
}

#[test]
fn undeclared() {
    let root = seat("undeclared", None);
    let (shown, held) = marked(&root, &[]);
    let _ = fs::remove_dir_all(&root);
    assert!(!held, "{shown}");
    assert!(shown.contains("no version to read"), "{shown}");
}

#[test]
fn invalid() {
    let root = seat("invalid", Some("1.2.3"));
    let (shown, held) = marked(&root, &["--version", "not-a-version"]);
    let _ = fs::remove_dir_all(&root);
    assert!(!held, "{shown}");
    assert!(shown.contains("invalid version"), "{shown}");
}
