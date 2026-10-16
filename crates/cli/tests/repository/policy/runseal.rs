use std::path::Path;
use std::process::Command;

fn rules(name: &str) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("git runs");
    assert!(output.status.success());
    let root = String::from_utf8(output.stdout).expect("git path");
    let path = Path::new(root.trim()).join("crates/cli/rules").join(name);
    std::fs::read_to_string(path).expect("carried rules")
}

fn include(files: &[&str]) -> Vec<String> {
    let root = tempfile::tempdir().expect("repository");
    let home = tempfile::tempdir().expect("rules home");
    for file in files {
        let path = root.path().join(file);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("source seat");
        std::fs::write(path, "").expect("source file");
    }
    super::govern(root.path());
    let scan = rules("atoms/scan.toml");
    let shape = rules("suites/shape.toml");
    super::stock(home.path(), super::LIMIT, &scan, &shape);
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .arg("policy")
        .arg(root.path())
        .arg("--write")
        .env("PLUMB_HOME", home.path())
        .output()
        .expect("plumb runs");
    assert!(output.status.success(), "{output:?}");
    let text = std::fs::read_to_string(root.path().join("ectropy.toml")).expect("policy");
    let policy: toml::Value = toml::from_str(&text).expect("policy parses");
    policy["scan"]["include"]
        .as_array()
        .expect("include")
        .iter()
        .map(|item| item.as_str().expect("glob").to_string())
        .collect()
}

#[test]
fn alone() {
    let held = include(&[".runseal/lib/cli.ts", "apps/web/package.json"]);
    assert!(held.contains(&".runseal/**".to_string()), "{held:?}");
}

#[test]
fn data() {
    let held = include(&[".runseal/deno.json", "crates/cli/src/lib.rs"]);
    assert!(!held.contains(&".runseal/**".to_string()), "{held:?}");
}

#[test]
fn absent() {
    let held = include(&["crates/cli/src/lib.rs"]);
    assert!(!held.contains(&".runseal/**".to_string()), "{held:?}");
}
