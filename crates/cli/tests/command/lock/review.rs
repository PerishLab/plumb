use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Output};

const APP: &str = "use std::fs::File as Handle;\ntype Alias = Handle;\npub fn alias(file: &Alias) { let _ = file.try_lock(); }\npub fn call(file: &std::fs::File) { let _ = carrier::protect!(file); let _ = carrier::protect!(file); }\n#[cfg(feature=\"extra\")]\npub fn extra(file: &std::fs::File) { let _ = file.lock_shared(); }\n";
const CARRIER: &str =
    "#[macro_export]\nmacro_rules! protect { ($file:expr) => { $file.try_lock() }; }\n";

fn put(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn stage(root: &Path) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["add", "."])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}

fn plant() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let path = root.path();
    put(
        path,
        "Cargo.toml",
        "[workspace]\nmembers=[\"app\",\"carrier\"]\nresolver=\"2\"\n",
    );
    put(
        path,
        "Cargo.lock",
        "version=4\n[[package]]\nname=\"probe\"\nversion=\"0.0.0\"\ndependencies=[\"carrier\"]\n[[package]]\nname=\"carrier\"\nversion=\"0.0.0\"\n",
    );
    put(path, "plumb.toml", "[identity]\nns=\"probe\"\n");
    put(
        path,
        "app/Cargo.toml",
        "[package]\nname=\"probe\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[features]\nextra=[]\n[dependencies]\ncarrier={path=\"../carrier\"}\n",
    );
    put(
        path,
        "carrier/Cargo.toml",
        "[package]\nname=\"carrier\"\nversion=\"0.0.0\"\nedition=\"2024\"\n",
    );
    put(path, "app/src/lib.rs", APP);
    put(path, "carrier/src/lib.rs", CARRIER);
    put(path, "protocol.txt", "fixture protocol revision one\n");
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    stage(path);
    root
}

struct Fixture<'a> {
    root: &'a Path,
    home: &'a Path,
}

impl Fixture<'_> {
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(["lock"])
            .args(args)
            .arg(self.root)
            .arg("--json")
            .env("PLUMB_HOME", self.home)
            .env_remove("SCCACHE_DIR")
            .env_remove("RUSTC_WRAPPER")
            .output()
            .unwrap()
    }

    fn policy(&self, value: &Value) {
        let value: toml::Value =
            serde_json::from_value(json!({"identity":{"ns":"probe"},"guard":{"lock":value}}))
                .unwrap();
        put(self.root, "plumb.toml", &toml::to_string(&value).unwrap());
        stage(self.root);
    }
}

fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn rejected(output: Output, reason: &str) {
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(reason),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn identities(report: &Value) -> BTreeMap<String, Value> {
    report["messages"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| {
            assert_eq!(finding["reference"]["state"], "bound", "{finding}");
            (
                finding["reference"]["digest"].as_str().unwrap().to_owned(),
                finding["reference"]["files"].clone(),
            )
        })
        .collect()
}

#[test]
fn review() {
    let root = plant();
    let home = tempfile::tempdir().unwrap();
    let path = root.path();
    let fixture = Fixture {
        root: path,
        home: home.path(),
    };
    rejected(fixture.run(&["check"]), "explicit [guard.lock]");
    let default = success(fixture.run(&["resolve"]));
    let all = success(fixture.run(&["resolve", "--all-features"]));
    let first = identities(&default);
    assert_eq!(
        first.len(),
        3,
        "alias and two macro callers need distinct identities"
    );
    assert!(
        default["messages"]["findings"].as_array().unwrap().len() > first.len(),
        "lib/test observations remain repeated"
    );
    let other = plant();
    let away = tempfile::tempdir().unwrap();
    let moved = success(
        Fixture {
            root: other.path(),
            home: away.path(),
        }
        .run(&["resolve"]),
    );
    assert_ne!(
        default["messages"]["findings"][0]["package"],
        moved["messages"]["findings"][0]["package"]
    );
    assert_eq!(
        identities(&moved),
        first,
        "reference identity must survive working directory relocation"
    );
    let mut bound = first.clone();
    let expanded = identities(&all);
    assert_eq!(expanded.len(), 4);
    assert!(
        first.keys().all(|key| !expanded.contains_key(key)),
        "each feature combination needs its own disposition"
    );
    bound.extend(expanded);
    let protocol = format!(
        "{:x}",
        Sha256::digest(std::fs::read(path.join("protocol.txt")).unwrap())
    );
    let reviews = bound.iter().map(|(id, files)| {
        let mut evidence = files.as_object().unwrap().clone();
        evidence.insert("protocol.txt".into(), Value::String(protocol.clone()));
        json!({"reference":id,"role":"interoperability","protocol":"native interop fixture","hold":"fixture handle lifetime","release":"fixture close or explicit cleanup","verification":"fixture assertions, not a lifetime proof","evidence":evidence})
    }).collect::<Vec<_>>();
    let at = default["argv"]
        .as_array()
        .unwrap()
        .iter()
        .position(|arg| arg == "--target")
        .unwrap();
    let valid = json!({"target":default["argv"][at+1],"features":["default","all-features"],"review":reviews});
    fixture.policy(&valid);
    let report = success(fixture.run(&["check"]));
    assert_eq!(report["consistent"], true);
    assert_eq!(report["guard"], false);
    assert_eq!(report["references"].as_array().unwrap().len(), 7);
    assert_eq!(report["reports"].as_array().unwrap().len(), 2);
    assert_eq!(report["declaration"]["review"], valid["review"]);
    let cached = success(fixture.run(&["check"]));
    assert_eq!(cached["references"], report["references"]);
    let mut changed = valid.clone();
    changed["review"] = json!([]);
    fixture.policy(&changed);
    rejected(fixture.run(&["check"]), "missing=");
    changed = valid.clone();
    let duplicate = changed["review"][0].clone();
    changed["review"].as_array_mut().unwrap().push(duplicate);
    fixture.policy(&changed);
    rejected(fixture.run(&["check"]), "unique SHA-256");
    changed = valid.clone();
    changed["review"][0]["reference"] = json!("a".repeat(64));
    fixture.policy(&changed);
    rejected(fixture.run(&["check"]), "stale=");
    changed = valid.clone();
    changed["features"] = json!(["default"]);
    fixture.policy(&changed);
    rejected(fixture.run(&["check"]), "stale=");
    changed = valid.clone();
    changed["features"] = json!(["default", "default"]);
    fixture.policy(&changed);
    rejected(fixture.run(&["check"]), "repeats a feature");
    changed = valid.clone();
    changed["target"] = json!("unexecuted-target");
    fixture.policy(&changed);
    rejected(fixture.run(&["check"]), "actual compilation host");
    changed = valid.clone();
    changed["review"][0]["evidence"] = json!({"app/**":"a".repeat(64)});
    fixture.policy(&changed);
    rejected(fixture.run(&["check"]), "exact relative files");
    changed = valid.clone();
    changed["review"][0]["hold"] = json!("");
    fixture.policy(&changed);
    rejected(fixture.run(&["check"]), "needs protocol");
    changed = valid.clone();
    changed["features"] = json!(["unsupported"]);
    fixture.policy(&changed);
    rejected(fixture.run(&["check"]), "invalid guard.lock");
    changed = valid.clone();
    changed["typo"] = json!(true);
    fixture.policy(&changed);
    rejected(fixture.run(&["check"]), "invalid guard.lock");
    fixture.policy(&valid);
    put(path, "protocol.txt", "fixture protocol revision two\n");
    stage(path);
    rejected(fixture.run(&["check"]), "evidence is stale");
    put(path, "protocol.txt", "fixture protocol revision one\n");
    put(
        path,
        "carrier/src/lib.rs",
        &format!("{CARRIER}pub fn changed() {{}}\n"),
    );
    stage(path);
    rejected(fixture.run(&["check"]), "evidence is stale");
    put(path, "carrier/src/lib.rs", CARRIER);
    stage(path);
    success(fixture.run(&["check"]));
    put(path, ".gitignore", "app/src/generated.rs\n");
    put(path, "app/src/lib.rs", "include!(\"generated.rs\");\n");
    put(
        path,
        "app/build.rs",
        r#"fn main() { std::fs::write("src/generated.rs", "pub fn generated(file: &std::fs::File) { let _ = file.try_lock(); }\n").unwrap(); }"#,
    );
    changed = valid.clone();
    changed["review"] = json!([]);
    fixture.policy(&changed);
    let generated = success(fixture.run(&["resolve"]));
    assert!(
        generated["messages"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["reference"]["state"] == "unknown")
    );
    rejected(fixture.run(&["check"]), "unbound compiler reference");
}
