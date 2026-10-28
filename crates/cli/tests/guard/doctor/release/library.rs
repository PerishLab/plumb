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

fn fixture(root: &Path) {
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers=['crates/*']\nresolver='3'\n",
    )
    .unwrap();
    for name in ["lock", "token"] {
        let path = root.join("crates").join(name);
        std::fs::create_dir_all(path.join("src")).unwrap();
        std::fs::write(
            path.join("Cargo.toml"),
            format!("[package]\nname='foo-{name}'\nversion='0.0.0'\nedition='2024'\n"),
        )
        .unwrap();
        std::fs::write(path.join("src/lib.rs"), "pub struct Available;\n").unwrap();
    }
    command(
        root,
        "git",
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/PerishLab/foo.git",
        ],
    );
    command(root, "cargo", &["generate-lockfile", "--offline"]);
}

fn report(root: &Path, targets: &str) -> serde_json::Value {
    let text = format!(
        "[identity]\nns='foo'\n[dependency]\nroot=true\n[release]\nauthority='https://example.invalid'\ntargets={targets}\n[release.cargo]\nregistry='perish'\npackages=['foo-lock','foo-token']\n"
    );
    std::fs::write(root.join("plumb.toml"), text).unwrap();
    command(root, "git", &["add", "."]);
    let home = crate::support::home();
    let output = crate::support::plumb()
        .args(["doctor", root.to_str().unwrap(), "--json"])
        .env("PLUMB_HOME", home.path())
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn platforms() {
    let (_scratch, root) = super::seat("library-platforms");
    fixture(&root);
    let targets = "['x86_64-unknown-linux-gnu','aarch64-apple-darwin','x86_64-pc-windows-msvc']";
    let held = report(&root, targets);
    let topology = &held["shape"]["topology"];
    assert_eq!(topology["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(topology["unknown"], serde_json::json!([]));
    assert_eq!(topology["refusals"], serde_json::json!([]));
    let findings = held["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .all(|finding| finding["code"] != "release.spec-declared"),
        "{held}"
    );
    assert_eq!(
        held["shape"]["publishes"],
        serde_json::json!(["cargo"]),
        "{held}"
    );
    for targets in [
        "['x86_64-apple-darwin']",
        "['x86_64-unknown-linux-gnu','x86_64-unknown-linux-gnu']",
    ] {
        let held = report(&root, targets);
        assert!(
            held["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|finding| finding["code"] == "release.spec-declared"
                    && finding["grade"] == "out of true"),
            "{held}"
        );
    }
}
