use std::process::Command;

#[test]
fn commands() {
    let binary = env!("CARGO_BIN_EXE_plumb");
    let help = Command::new(binary)
        .arg("--help")
        .output()
        .expect("top-level help");
    assert!(help.status.success());
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("land"));
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("plan"))
    );

    let removed = Command::new(binary)
        .args(["plan", "delivery", "--help"])
        .output()
        .expect("removed plan command");
    assert!(!removed.status.success());

    let land = Command::new(binary)
        .args(["land", "--help"])
        .output()
        .expect("repository landing help");
    assert!(land.status.success());
    let land = String::from_utf8_lossy(&land.stdout);
    assert!(land.contains("--title"));
    assert!(land.contains("--body"));
    assert!(!land.contains("--plan"));
}

#[test]
fn inspection() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("record.json");
    let mut record = serde_json::json!({
        "schema":"wharf.release.semantics.publication/v1",
        "product":"plumb", "marker":"v0.139.0",
        "intent":{"url":format!("https://releases.plumb.perish.uk/v1/semantics/objects/{}.json", "a".repeat(64)),"sha256":"a".repeat(64),"size":12},
        "world":{"url":format!("https://releases.plumb.perish.uk/v1/semantics/objects/{}.json", "b".repeat(64)),"sha256":"b".repeat(64),"size":34},
        "native":{"run":7,"attempt":1,"job":9,"commit":"c".repeat(40)}
    });
    record.sort_all_objects();
    std::fs::write(&input, serde_json::to_vec(&record).unwrap()).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["release", "inspect", "--input"])
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let held: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(held["state"], "observed");
    assert_eq!(held["publication"], false);
    assert_eq!(held["facts"]["authentication"], false);
    assert_eq!(held["facts"]["publication"], false);
    assert_eq!(held["facts"]["data"], record);
    for field in ["authorized", "unknown"] {
        let mut changed = record.clone();
        changed[field] = true.into();
        std::fs::write(&input, serde_json::to_vec(&changed).unwrap()).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["release", "inspect", "--input"])
            .arg(&input)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        let held: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(held["publication"], false);
    }
}

#[test]
fn purposes() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("reference.json");
    let reference = serde_json::json!({"url":"https://foreign.example/object.json","sha256":"a".repeat(64),"size":12});
    std::fs::write(&input, serde_json::to_vec(&reference).unwrap()).unwrap();
    for purpose in ["issuer", "baseline"] {
        let result = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["release", "inspect", "--purpose", purpose, "--input"])
            .arg(&input)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&result.stderr).contains("--product"));
        let result = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args([
                "release",
                "inspect",
                "--purpose",
                purpose,
                "--product",
                "plumb",
                "--input",
            ])
            .arg(&input)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        let held: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(held["state"], "refused");
        assert_eq!(held["publication"], false);
        assert!(
            held["reason"]
                .as_str()
                .unwrap()
                .contains("foreign authority")
        );
    }
    let result = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["release", "inspect", "--purpose", "materials", "--input"])
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    let result = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "release",
            "inspect",
            "--purpose",
            "materials",
            "--product",
            "plumb",
            "--input",
        ])
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let held: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(held["state"], "refused");
    assert_eq!(held["publication"], false);
    let result = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["release", "inspect", "--product", "plumb", "--input"])
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let held: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(held["state"], "refused");
    assert!(
        held["reason"]
            .as_str()
            .unwrap()
            .contains("does not select a product")
    );
}

#[test]
fn preparation() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("preparation.json");
    std::fs::write(&input, b"{}").unwrap();
    let command = || {
        let mut held = Command::new(env!("CARGO_BIN_EXE_plumb"));
        held.args(["release", "inspect", "--purpose", "preparation", "--input"]);
        held.arg(&input);
        held
    };
    assert_eq!(command().output().unwrap().status.code(), Some(2));
    let reference = serde_json::json!({"url":format!("https://releases.plumb.perish.uk/v1/semantics/objects/{}.json", "a".repeat(64)),"sha256":"a".repeat(64),"size":12});
    for value in [
        serde_json::json!({}),
        serde_json::json!({"schema":"wharf.release.semantics.world/v1", "native":{"run":1,"attempt":1,"job":1,"commit":"a".repeat(40)},"world":reference,"material":reference,"authentication":true}),
    ] {
        std::fs::write(&input, serde_json::to_vec(&value).unwrap()).unwrap();
        let result = command().args(["--product", "plumb"]).output().unwrap();
        assert_eq!(result.status.code(), Some(1));
        let held: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(held["state"], "refused");
        assert_eq!(held["publication"], false);
    }
}

#[test]
fn history() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("history.json");
    std::fs::write(&input, b"{\"authentication\":true}").unwrap();
    for purpose in ["history", "retention"] {
        let command = || {
            let mut held = Command::new(env!("CARGO_BIN_EXE_plumb"));
            held.args(["release", "inspect", "--purpose", purpose, "--input"]);
            held.arg(&input);
            held
        };
        assert_eq!(command().output().unwrap().status.code(), Some(2));
        let result = command().args(["--product", "plumb"]).output().unwrap();
        assert_eq!(result.status.code(), Some(1));
        let held: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(held["state"], "refused");
        assert_eq!(held["publication"], false);
    }
    let reference = serde_json::json!({"url":format!("https://releases.plumb.perish.uk/v1/semantics/objects/{}.json", "a".repeat(64)),"sha256":"a".repeat(64),"size":12});
    let mut checkpoint = serde_json::json!({"schema":"wharf.release.semantics.history/v1","product":"plumb","repository":{"id":1,"node_id":"R_fixture","full_name":"PerishLab/plumb"},"root":reference,"previous":null,"sequence":1,"markers":[{"marker":"v0.1.0","commit":"a".repeat(40)}],"native":{"run":1,"attempt":1,"commit":"a".repeat(40),"job":1}});
    checkpoint.sort_all_objects();
    std::fs::write(&input, serde_json::to_vec(&checkpoint).unwrap()).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["release", "inspect", "--input"])
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    let held: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(held["facts"]["authentication"], false);
    assert_eq!(held["facts"]["publication"], false);
}
