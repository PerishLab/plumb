#[path = "../../../../src/command/ship/request/projection.rs"]
mod implementation;

use serde_json::json;

#[test]
fn committed() {
    let root = tempfile::tempdir().unwrap();
    let run = |arguments: &[&str]| {
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(root.path())
                .args(arguments)
                .status()
                .unwrap()
                .success()
        );
    };
    run(&["init", "-q"]);
    std::fs::write(root.path().join("Cargo.toml"), "version='1'").unwrap();
    run(&["add", "Cargo.toml"]);
    run(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.test",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "source",
    ]);
    std::fs::write(root.path().join("Cargo.toml"), "uncommitted").unwrap();
    let recipe = json!({"format":"toml","set":{"/version":"0"}});
    let tree = json!({"paths":["Cargo.toml"],"projects":{"Cargo.toml":recipe}});
    let inputs = json!({"source":{"tree":tree}});
    let mut graph = json!({"nodes":[{"inputs":inputs}]});
    implementation::resolve(&mut graph, root.path(), "HEAD").unwrap();
    assert_eq!(
        graph["nodes"][0]["inputs"]["source"]["tree"]["patches"]["Cargo.toml"]["source"],
        plumb::depot::sha(b"version='1'")
    );
    bridge(root.path(), &graph["nodes"][0]["inputs"]["source"]["tree"]);
}

fn bridge(root: &std::path::Path, tree: &serde_json::Value) {
    let recipe = root.join("recipe.json");
    std::fs::write(&recipe, tree.to_string()).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(source.join(".forgejo/scripts/input.py"))
        .arg("--recipe")
        .arg(recipe)
        .arg("--root")
        .arg(root)
        .arg("--destination")
        .arg(root.join("materialized"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual = std::fs::read_to_string(root.join("materialized/Cargo.toml")).unwrap();
    let value: toml::Value = toml::from_str(&actual).unwrap();
    assert_eq!(value["version"].as_str(), Some("0"));
}

fn material(body: &str, recipe: serde_json::Value) -> String {
    let patch = implementation::patch(body, &recipe).unwrap();
    let mut actual = String::new();
    let mut offset = 0;
    for edit in patch["edits"].as_array().unwrap() {
        let start = edit[0].as_u64().unwrap() as usize;
        actual.push_str(&body[offset..start]);
        actual.push_str(edit[2].as_str().unwrap());
        offset = edit[1].as_u64().unwrap() as usize;
    }
    actual.push_str(&body[offset..]);
    assert_eq!(patch["source"], plumb::depot::sha(body.as_bytes()));
    assert_eq!(patch["result"], plumb::depot::sha(actual.as_bytes()));
    actual
}

#[test]
fn identity() {
    let recipe = json!({"format":"toml","set":{"/package/version":"0.0.0"}});
    let first = "[package]\nname='🦀'\nversion='1'\n[dependencies]\nexternal='9'\n";
    let next = first.replace("version='1'", "version='20.30.400-beta.5'");
    let actual = material(first, recipe.clone());
    assert_eq!(actual, material(&next, recipe));
    let value: toml::Value = toml::from_str(&actual).unwrap();
    assert_eq!(value["package"]["version"].as_str(), Some("0.0.0"));
    assert_eq!(value["dependencies"]["external"].as_str(), Some("9"));
    assert!(actual.contains("name='🦀'"));
}

#[test]
fn arrays() {
    let recipe = json!({"format":"toml","set":{"/package/0/version":"0.0.0"}});
    let actual = material(
        "[[package]]\nname='a'\nversion='1'\ndependencies=['b']\n",
        recipe,
    );
    let value: toml::Value = toml::from_str(&actual).unwrap();
    assert_eq!(value["package"][0]["version"].as_str(), Some("0.0.0"));
    assert_eq!(value["package"][0]["dependencies"][0].as_str(), Some("b"));
}

#[test]
fn refuses() {
    for sets in [
        json!({"/missing":"0"}),
        json!({"/version":null}),
        json!({"/version/0":"0"}),
        json!({"/bad~2":"0"}),
    ] {
        assert!(
            implementation::patch("version='1'", &json!({"format":"toml","set":sets})).is_err()
        );
    }
}
