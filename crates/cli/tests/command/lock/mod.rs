mod carrier;

use std::path::Path;
use std::process::Command;

const MANIFEST: &str = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

const CASES: [(&str, &str, &str); 12] = [
    (
        "custody",
        "use std::fs::File;\npub fn hold(path: &std::path::Path) -> std::io::Result<File> {\n    let file = match File::create(path) {\n        Ok(file) => file,\n        Err(error) => return Err(error),\n    };\n    file.try_lock().map_err(std::io::Error::other)?;\n    Ok(file)\n}\n",
        "src/lib.rs:7:10 primitive[acquire] std::fs::File::try_lock hold=handle (production)",
    ),
    (
        "alias",
        "use std::sync::Mutex;\ntype Gate = Mutex<()>;\npub static GATE: Gate = Mutex::new(());\npub fn zone() {\n    let _held = GATE.lock();\n}\n",
        "src/lib.rs:5:22 candidate[acquire] std::sync::Mutex::lock hold=binding (production)",
    ),
    (
        "extension",
        "use fs4::fs_std::FileExt;\npub fn hold(file: &std::fs::File) {\n    file.lock_exclusive().unwrap();\n}\n",
        "src/lib.rs:3:10 primitive[acquire] fs4::fs_std::FileExt::lock_exclusive hold=handle (production)",
    ),
    (
        "awaited",
        "use std::sync::Arc;\nuse tokio::sync::Mutex;\npub struct Service {\n    gate: Arc<Mutex<()>>,\n}\nimpl Service {\n    pub async fn run(&self) {\n        let _held = self.gate.lock().await;\n        work().await;\n    }\n}\nasync fn work() {}\n",
        "src/lib.rs:8:31 candidate[acquire] tokio::sync::Mutex::lock hold=binding (production)",
    ),
    (
        "data",
        "use std::sync::{Arc, Mutex};\npub struct Connection;\npub struct Store {\n    connection: Arc<Mutex<Connection>>,\n}\nimpl Store {\n    pub fn read(&self) {\n        let _connection = self.connection.lock().unwrap();\n    }\n}\n",
        "src/lib.rs:8:43 data[acquire] std::sync::Mutex::lock hold=binding (production)",
    ),
    (
        "manual",
        "pub fn hold(path: &std::path::Path) {\n    radicula_lock::lock(path).unwrap();\n    radicula_lock::unlock(path).unwrap();\n}\n",
        "src/lib.rs:2:5 radicula[manual] radicula_lock::lock hold=manual (production)",
    ),
    (
        "stdio",
        "use std::io::Write;\npub fn say() {\n    let mut out = std::io::stdout().lock();\n    let _ = writeln!(out, \"Cargo.lock\");\n}\n",
        "src/lib.rs:3:37 stdio[acquire] std::io::Stdout::lock (production)",
    ),
    (
        "feature",
        "#[cfg(feature = \"gate\")]\npub fn hold(file: &std::fs::File) {\n    file.lock().unwrap();\n}\n",
        "src/lib.rs:3:10 primitive[acquire] std::fs::File::lock hold=handle cfg(feature = \"gate\") (production)",
    ),
    (
        "target",
        "#[cfg(unix)]\npub fn hold(file: &std::fs::File) {\n    file.lock_shared().unwrap();\n}\n",
        "src/lib.rs:3:10 primitive[acquire] std::fs::File::lock_shared hold=handle cfg(unix) (production)",
    ),
    (
        "test",
        "#[cfg(test)]\nmod tests {\n    #[test]\n    fn hold() {\n        let file = std::fs::File::create(\"probe\").unwrap();\n        file.unlock().unwrap();\n    }\n}\n",
        "src/lib.rs:6:14 primitive[release] std::fs::File::unlock cfg(test) (test)",
    ),
    (
        "unknown",
        "pub fn hold<T: Clone>(gate: T) {\n    gate.lock();\n}\n",
        "src/lib.rs:2:5 unknown lock-named method `lock` on a receiver whose type is not inferred",
    ),
    (
        "hidden",
        "macro_rules! seal {\n    ($body:tt) => {};\n}\npub fn hold() {\n    seal!({ gate.lock() });\n}\n",
        "unknown",
    ),
];

fn inspect(root: &Path, json: bool) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command.args([
        "lock",
        "inspect",
        root.to_str().expect("path should be utf8"),
    ]);
    if json {
        command.arg("--json");
    }
    let output = command
        .env("CARGO_HOME", root.join(".cargo"))
        .env("PLUMB_HOME", root.join(".plumb"))
        .output()
        .expect("plumb should run");
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).expect("output should be utf8")
}

fn plant(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("fixture should be made");
    for (path, text) in files {
        let place = root.path().join(path);
        std::fs::create_dir_all(place.parent().expect("file should have a parent"))
            .expect("directory should be made");
        std::fs::write(place, text).expect("file should be written");
    }
    root
}

#[test]
fn cases() {
    for (name, source, expected) in CASES {
        let root = plant(&[("Cargo.toml", MANIFEST), ("src/lib.rs", source)]);
        let text = inspect(root.path(), false);
        assert!(
            text.lines().any(|line| line.contains(expected)),
            "{name}\n{text}"
        );
    }
}

#[test]
fn paired() {
    let (_, source, _) = CASES[5];
    let root = plant(&[("Cargo.toml", MANIFEST), ("src/lib.rs", source)]);
    let result: serde_json::Value =
        serde_json::from_str(&inspect(root.path(), true)).expect("inspection should be json");
    let manual = &result["findings"][0];
    assert_eq!(manual["lifecycle"], "manual", "{result:#}");
    assert_eq!(manual["hold"]["release"], 3, "{result:#}");
    assert_eq!(result["findings"][1]["lifecycle"], "release", "{result:#}");
}

#[test]
fn awaited() {
    let (_, source, _) = CASES[3];
    let root = plant(&[("Cargo.toml", MANIFEST), ("src/lib.rs", source)]);
    let result: serde_json::Value =
        serde_json::from_str(&inspect(root.path(), true)).expect("inspection should be json");
    let hold = &result["findings"][1]["hold"];
    assert_eq!(hold["across"], true, "{result:#}");
    assert_eq!(hold["awaited"], true, "{result:#}");
    assert_eq!(
        result["findings"][1]["domain"]["owner"], "demo::Service::gate",
        "{result:#}"
    );
}

#[test]
fn indirect() {
    let root = plant(&[
        (
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\"]\nresolver = \"3\"\n",
        ),
        (
            "crates/store/Cargo.toml",
            "[package]\nname = \"store\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        (
            "crates/store/src/lib.rs",
            "pub fn claim(path: &std::path::Path) -> std::io::Result<std::fs::File> {\n    let file = std::fs::File::create(path)?;\n    file.try_lock().map_err(std::io::Error::other)?;\n    Ok(file)\n}\n",
        ),
        (
            "crates/app/Cargo.toml",
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[dependencies]\nstore = { path = \"../store\" }\n",
        ),
        (
            "crates/app/src/main.rs",
            "fn main() {\n    let _file = store::claim(std::path::Path::new(\"seat\"));\n}\n",
        ),
    ]);
    let result: serde_json::Value =
        serde_json::from_str(&inspect(root.path(), true)).expect("inspection should be json");
    let found = result["findings"]
        .as_array()
        .expect("findings should be listed")
        .iter()
        .find(|finding| finding["role"] == "indirect")
        .expect("indirect consumption should be found");
    assert_eq!(found["path"], "crates/app/src/main.rs", "{result:#}");
    assert_eq!(found["call"], "store::claim", "{result:#}");
    assert_eq!(
        found["via"][0], "std::fs::File::try_lock at crates/store/src/lib.rs:3",
        "{result:#}"
    );
    assert_eq!(result["summary"]["complete"], true, "{result:#}");
}

#[test]
fn quiet() {
    let source = "use std::os::unix::fs::FileExt;\npub const SCRIPT: &str = \"import fcntl\\nfcntl.flock(handle, fcntl.LOCK_EX)\";\npub fn read(file: &std::fs::File, path: &std::path::Path) {\n    let mut buffer = [0; 4];\n    let _ = file.read_at(&mut buffer, 0);\n    let _ = path.join(\"Cargo.lock\");\n}\n";
    let root = plant(&[
        ("Cargo.toml", MANIFEST),
        ("src/lib.rs", source),
        ("Cargo.lock", "version = 4\n"),
    ]);
    let result: serde_json::Value =
        serde_json::from_str(&inspect(root.path(), true)).expect("inspection should be json");
    assert_eq!(result["findings"], serde_json::json!([]), "{result:#}");
    assert_eq!(result["summary"]["complete"], true, "{result:#}");
}

#[test]
fn foreign() {
    let root = plant(&[
        ("Cargo.toml", MANIFEST),
        ("src/lib.rs", "pub fn idle() {}\n"),
        ("tools/seal.py", "import fcntl\n"),
        ("tests/fixtures/sample/Cargo.toml", MANIFEST),
        (
            "tests/fixtures/sample/src/lib.rs",
            "pub fn hold(file: &std::fs::File) {\n    file.lock().unwrap();\n}\n",
        ),
    ]);
    let result: serde_json::Value =
        serde_json::from_str(&inspect(root.path(), true)).expect("inspection should be json");
    assert_eq!(
        result["unscanned"][0]["path"], "tools/seal.py",
        "{result:#}"
    );
    assert_eq!(result["summary"]["complete"], false, "{result:#}");
    let fixture = &result["findings"][0];
    assert_eq!(fixture["context"], "fixture", "{result:#}");
    assert_eq!(fixture["role"], "primitive", "{result:#}");
}

#[test]
fn stable() {
    let (_, source, _) = CASES[4];
    let root = plant(&[
        ("Cargo.toml", MANIFEST),
        ("src/lib.rs", &format!("{source}mod extra;\n")),
        ("src/extra.rs", "include!(\"part.rs\");\n"),
    ]);
    let first = inspect(root.path(), true);
    assert_eq!(first, inspect(root.path(), true));
    let result: serde_json::Value =
        serde_json::from_str(&first).expect("inspection should be json");
    assert_eq!(result["schema"], "plumb.lock.inspection/v1", "{result:#}");
    assert_eq!(result["tool"]["parser"], "syn 2", "{result:#}");
    assert_eq!(result["summary"]["roles"]["data"], 2, "{result:#}");
    assert!(
        result["unknown"]
            .as_array()
            .is_some_and(|gaps| !gaps.is_empty()),
        "{result:#}"
    );
}

#[test]
fn poison() {
    let source = "use std::sync::Mutex;\nstatic GATE: Mutex<()> = Mutex::new(());\npub fn append() -> Result<(), &'static str> {\n    let _held = GATE.lock().map_err(|_| \"store lock is poisoned\")?;\n    Ok(())\n}\n";
    let root = plant(&[("Cargo.toml", MANIFEST), ("src/lib.rs", source)]);
    let result: serde_json::Value =
        serde_json::from_str(&inspect(root.path(), true)).expect("inspection should be json");
    let findings = result["findings"].as_array().expect("findings");
    assert!(
        findings
            .iter()
            .any(|finding| finding["role"] == "candidate")
    );
    assert!(!findings.iter().any(|finding| finding["role"] == "zone"));
    assert_eq!(result["summary"]["complete"], false);
    assert!(
        result["unknown"]
            .as_array()
            .expect("gaps")
            .iter()
            .any(|gap| {
                gap["reason"]
                    .as_str()
                    .is_some_and(|reason| reason.contains("poison protocols"))
            })
    );
}
