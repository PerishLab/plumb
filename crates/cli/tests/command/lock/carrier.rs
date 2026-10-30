use super::{MANIFEST, inspect, plant};

#[test]
fn file() {
    let source = "use radicula_lock::{Factory, Policy};\npub fn run(file: std::fs::File) {\n    let gate = Factory::file(file, Policy::EXCLUSIVE).unwrap();\n    gate.guard(|| ()).unwrap();\n}\n";
    let root = plant(&[("Cargo.toml", MANIFEST), ("src/lib.rs", source)]);
    let result: serde_json::Value =
        serde_json::from_str(&inspect(root.path(), true)).expect("inspection should be json");
    let findings = result["findings"].as_array().expect("findings");
    assert!(
        findings
            .iter()
            .any(|finding| finding["call"] == "radicula_lock::Factory::file"
                && finding["lifecycle"] == "gate")
    );
    assert!(findings.iter().any(
        |finding| finding["call"] == "radicula_lock::file::Gate::guard"
            && finding["hold"]["form"] == "scoped"
    ));
    assert_eq!(result["summary"]["complete"], true);
}

#[test]
fn future() {
    let root = plant(&[
        ("Cargo.toml", MANIFEST),
        (
            "src/lib.rs",
            "pub fn run() { radicula_lock::Factory::future(); }\n",
        ),
    ]);
    let result: serde_json::Value =
        serde_json::from_str(&inspect(root.path(), true)).expect("inspection should be json");
    assert_eq!(result["summary"]["complete"], false);
    assert!(
        result["unknown"]
            .as_array()
            .expect("gaps")
            .iter()
            .any(|gap| gap["reason"]
                .as_str()
                .is_some_and(|reason| reason.contains("unmodeled Radicula call")))
    );
}

#[test]
fn opaque() {
    let source = "pub fn run() { external::protect!(); }\n";
    let root = plant(&[("Cargo.toml", MANIFEST), ("src/lib.rs", source)]);
    let result: serde_json::Value =
        serde_json::from_str(&inspect(root.path(), true)).expect("inspection should be json");
    assert_eq!(result["summary"]["complete"], false);
    assert!(
        result["unknown"]
            .as_array()
            .expect("gaps")
            .iter()
            .any(|gap| gap["reason"]
                .as_str()
                .is_some_and(|reason| reason.contains("macro expansion is not inspected")))
    );
}

#[test]
fn platform() {
    let root = plant(&[
        ("Cargo.toml", MANIFEST),
        ("src/lib.rs", "pub fn idle() {}\n"),
        (
            "tools/exact.ps1",
            "$mutex = [System.Threading.Mutex]::new()\n",
        ),
        ("src/native.m", "@synchronized (self) {}\n"),
        ("src/native.uc", "const file = open('state', 'r');\n"),
    ]);
    let result: serde_json::Value =
        serde_json::from_str(&inspect(root.path(), true)).expect("inspection should be json");
    assert_eq!(result["summary"]["complete"], false);
    let skips = result["unscanned"].as_array().expect("unscanned");
    assert_eq!(skips.len(), 3);
    assert!(skips.iter().any(|skip| skip["path"] == "tools/exact.ps1"));
}
