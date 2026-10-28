use std::path::Path;

fn command(root: &Path, program: &str, args: &[&str]) {
    let output = crate::support::outside(program)
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture() -> tempfile::TempDir {
    let root = super::super::fixture();
    let path = root.path();
    let declaration = std::fs::read_to_string(path.join("plumb.toml")).unwrap();
    super::write(
        path,
        "plumb.toml",
        &format!("{declaration}\n[dependency]\nroot=true\n"),
    );
    super::write(
        path,
        "Cargo.toml",
        "[workspace]\nmembers=['crates/*']\nresolver='3'\n",
    );
    for name in ["lock", "peer"] {
        super::write(
            path,
            &format!("crates/{name}/Cargo.toml"),
            &format!("[package]\nname='{name}'\nversion='0.0.0'\nedition='2024'\n"),
        );
        super::write(path, &format!("crates/{name}/src/lib.rs"), "");
    }
    command(
        path,
        "git",
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/PerishLab/radicula.git",
        ],
    );
    command(path, "cargo", &["generate-lockfile", "--offline"]);
    command(path, "git", &["add", "."]);
    root
}

fn observe(root: &Path) -> serde_json::Value {
    let home = crate::support::home();
    let output = crate::support::plumb()
        .args(["doctor", root.to_str().unwrap(), "--json"])
        .env_remove("PLUMB_RELEASE_VERSION")
        .env("PLUMB_HOME", home.path())
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn independent() {
    let root = fixture();
    let report = observe(root.path());
    let topology = &report["shape"]["topology"];
    assert_eq!(topology["root"], true, "{report}");
    assert_eq!(topology["unknown"], serde_json::json!([]), "{report}");
    assert_eq!(topology["refusals"], serde_json::json!([]), "{report}");
    assert_eq!(topology["nodes"].as_array().unwrap().len(), 2);
    assert!(topology["input"].as_str().unwrap().len() == 64);
    assert!(
        !report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| { finding["code"] == "structure.anchor-present" }),
        "{report}"
    );
}

#[test]
fn ordinary() {
    let root = fixture();
    let declaration = root.path().join("plumb.toml");
    let held = std::fs::read_to_string(&declaration).unwrap();
    for text in [
        held.replace("\n[dependency]\nroot=true\n", "\n"),
        held.replace("root=true", "root=false"),
    ] {
        std::fs::write(&declaration, text).unwrap();
        let report = observe(root.path());
        assert!(
            report["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|finding| { finding["code"] == "structure.anchor-present" }),
            "{report}"
        );
    }
}

#[test]
fn platform() {
    let root = fixture();
    let before = observe(root.path())["shape"]["topology"]["input"].clone();
    let manifest = root.path().join("crates/lock/Cargo.toml");
    let held = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        &manifest,
        format!("{held}\n[target.'cfg(windows)'.dev-dependencies]\npeer={{path='../peer'}}\n"),
    )
    .unwrap();
    command(root.path(), "cargo", &["generate-lockfile", "--offline"]);
    let report = observe(root.path());
    let topology = &report["shape"]["topology"];
    assert_ne!(topology["input"], before);
    assert_eq!(topology["unknown"], serde_json::json!([]), "{report}");
    let refusals = topology["refusals"].as_array().unwrap();
    assert_eq!(refusals.len(), 1, "{report}");
    assert_eq!(refusals[0]["path"][0]["kind"], "dev");
    assert_eq!(refusals[0]["path"][0]["target"], "cfg(windows)");
    assert!(report.to_string().contains("deps.root-library-closed"));
}

#[test]
fn incomplete() {
    let root = fixture();
    std::fs::remove_file(root.path().join("Cargo.lock")).unwrap();
    let report = observe(root.path());
    assert!(
        !report["shape"]["topology"]["unknown"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{report}"
    );
    assert!(report.to_string().contains("deps.root-library-closed"));
    assert!(!root.path().join("Cargo.lock").exists());
}

#[test]
fn identity() {
    let root = fixture();
    let before = observe(root.path())["shape"]["topology"]["input"].clone();
    command(
        root.path(),
        "git",
        &[
            "remote",
            "set-url",
            "origin",
            "https://github.com/PerishLab/another.git",
        ],
    );
    let report = observe(root.path());
    assert_ne!(report["shape"]["topology"]["input"], before);
    let declaration = root.path().join("plumb.toml");
    let held = std::fs::read_to_string(&declaration).unwrap();
    std::fs::write(declaration, held.replace("root=true", "root=false")).unwrap();
    let report = observe(root.path());
    assert!(report["shape"]["topology"]["root"].is_null());
    assert!(
        !report["shape"]["topology"]["unknown"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(report.to_string().contains("deps.root-library-closed"));
}
