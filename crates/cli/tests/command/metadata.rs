use std::process::Command;

fn metadata(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", tempfile::tempdir().expect("home").keep())
        .arg("metadata")
        .args(args)
        .output()
        .expect("plumb should run");
    (
        output.status.code().expect("plumb should exit"),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn keys() {
    for (key, version) in [
        ("node.version", "24.18.0"),
        ("pnpm.version", "11.13.0"),
        ("rust.version", "1.96.1"),
    ] {
        assert_eq!(metadata(&[key]), (0, format!("{version}\n"), String::new()));
    }
}

#[test]
fn json() {
    let (code, out, _) = metadata(&["--json"]);
    assert_eq!(code, 0);
    let held: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(held["node.version"], "24.18.0");
    assert_eq!(held["pnpm.version"], "11.13.0");
    assert_eq!(held["rust.version"], "1.96.1");
}

#[test]
fn unknown() {
    let (code, out, err) = metadata(&["node"]);
    assert_eq!((code, out.as_str()), (2, ""));
    assert!(err.contains("unknown key node"), "{err}");
    assert_eq!(metadata(&[]).0, 2);
}
