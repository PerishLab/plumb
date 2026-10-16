use super::{govern, policy, run};
use std::path::{Path, PathBuf};

const TYPES: &str = "[[layout.seat]]\npath = \"types\"\nnote = \"declarations\"\n";

fn fixture(scratch: &Path, name: &str, layout: &str) -> PathBuf {
    let root = scratch.join(name);
    std::fs::create_dir_all(root.join("crates/cli/src")).expect("fixture should be made");
    std::fs::create_dir_all(root.join("types")).expect("fixture should be made");
    std::fs::write(root.join("crates/cli/src/lib.rs"), "").expect("fixture should be made");
    std::fs::write(root.join("types/host.d.ts"), "").expect("fixture should be made");
    std::fs::write(root.join("plumb.toml"), layout).expect("layout should be written");
    std::fs::write(root.join("ectropy.toml"), "").expect("policy should be written");
    govern(&root);
    root
}

fn rendered(root: &Path) -> toml::Value {
    let output = policy(root, true);
    assert!(output.status.success(), "{output:?}");
    let text = std::fs::read_to_string(root.join("ectropy.toml")).expect("rendered policy");
    toml::from_str(&text).expect("policy parses")
}

fn listed<'a>(policy: &'a toml::Value, table: &str, key: &str) -> Vec<&'a str> {
    policy[table][key]
        .as_array()
        .expect("list")
        .iter()
        .map(|item| item.as_str().expect("path"))
        .collect()
}

#[test]
fn declared() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let layout = format!("{TYPES}scan = [\"types/**/*.d.ts\"]\n");
    let root = fixture(scratch.path(), "plumb-seat-declared", &layout);
    let held = rendered(&root);
    assert_eq!(
        listed(&held, "scan", "include"),
        ["crates/**/*.rs", "types/**/*.d.ts"]
    );
    assert!(listed(&held, "module", "roots").contains(&"types"));
    let out = run(&root);
    assert!(!out.contains("unexpected ectropy"), "{out}");
    assert!(!out.contains("missing ectropy scan"), "{out}");
    assert!(
        out.contains("types/**/*.d.ts is declared by layout seat types"),
        "{out}"
    );
}

#[test]
fn outside() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let layout = format!("{TYPES}scan = [\"tools/**/*.rs\"]\n");
    let root = fixture(scratch.path(), "plumb-seat-outside", &layout);
    let out = run(&root);
    assert!(
        out.contains("layout seat types scan tools/**/*.rs leaves its seat"),
        "{out}"
    );
}

#[test]
fn undeclared() {
    let scratch = tempfile::tempdir().expect("fixture parent should be made");
    let root = fixture(scratch.path(), "plumb-seat-undeclared", TYPES);
    let held = rendered(&root);
    assert_eq!(listed(&held, "scan", "include"), ["crates/**/*.rs"]);
    assert!(!listed(&held, "module", "roots").contains(&"types"));
    assert!(!run(&root).contains("declared by layout seat"));
}
