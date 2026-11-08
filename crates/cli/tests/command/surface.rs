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
