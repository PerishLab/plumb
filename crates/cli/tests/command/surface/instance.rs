use std::process::Command;

#[test]
fn help() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["instance", "--json", "--help"])
        .env("PLUMB_HOME", home.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("not proof"));
}

#[test]
fn refusal() {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["instance", "--path", "fixture.pub"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected argument"));
}

fn run(home: &std::path::Path, args: &[&str], variable: Option<&str>) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command
        .arg("instance")
        .args(args)
        .env("PLUMB_HOME", home)
        .env_remove("PLUMB_INSTANCE_ID");
    if let Some(value) = variable {
        command.env("PLUMB_INSTANCE_ID", value);
    }
    command.output().unwrap()
}

fn json(output: &std::process::Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn declared() {
    let home = tempfile::tempdir().unwrap();
    std::fs::write(
        home.path().join("plumb.toml"),
        "instance_id = \"filed\"\n[locus]\nenabled = false\n",
    )
    .unwrap();
    let filed = json(&run(home.path(), &["--json"], None));
    assert_eq!(filed["schema"], "plumb.instance/v1");
    assert_eq!(filed["id"], "filed");
    assert_eq!(filed["source"]["kind"], "configuration");
    assert_eq!(
        filed["source"]["path"],
        home.path().join("plumb.toml").to_str().unwrap()
    );
    let enved = json(&run(home.path(), &["--json"], Some("enved")));
    assert_eq!(enved["id"], "enved");
    assert_eq!(enved["source"]["kind"], "environment");
    assert!(enved["source"].get("path").is_none());
    let argued = json(&run(
        home.path(),
        &["--json", "--instance-id", "argued"],
        Some("enved"),
    ));
    assert_eq!(argued["id"], "argued");
    assert_eq!(argued["source"]["kind"], "argument");
    assert!(argued["source"].get("path").is_none());
}

#[test]
fn spoken() {
    let home = tempfile::tempdir().unwrap();
    let output = run(home.path(), &["--instance-id", "plain value"], None);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "plain value\nsource: argument\n"
    );
    std::fs::write(home.path().join("plumb.toml"), "instance_id = \"filed\"\n").unwrap();
    let output = run(home.path(), &[], None);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!(
            "filed\nsource: configuration {}\n",
            home.path().join("plumb.toml").display()
        )
    );
}

#[test]
fn refused() {
    let home = tempfile::tempdir().unwrap();
    for (args, variable, name) in [
        (vec!["--instance-id", ""], None, "--instance-id"),
        (vec!["--instance-id", " x"], Some("valid"), "--instance-id"),
        (vec![], Some("a\u{1}b"), "PLUMB_INSTANCE_ID"),
    ] {
        let output = run(home.path(), &args, variable);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(name), "{error}");
        assert!(error.contains("instance id"), "{error}");
    }
    std::fs::write(home.path().join("plumb.toml"), "instance_id = [\n").unwrap();
    let output = run(home.path(), &["--instance-id", "argued"], None);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("plumb.toml"));
}

#[test]
fn described() {
    let home = tempfile::tempdir().unwrap();
    let output = run(home.path(), &["--help"], None);
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("--instance-id"));
    assert!(text.contains("PLUMB_INSTANCE_ID"));
}
