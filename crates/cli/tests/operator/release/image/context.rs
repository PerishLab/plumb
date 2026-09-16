use super::super::world::{Fixture, run};
use super::{executable, seat};
use std::collections::BTreeMap;
use std::io::Read;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;

const SPEC: &str = r#"
[release]
product = "probe"
authority = "https://releases.test"
[release.oci]
registry = "registry.example"
image = "owner/probe"
account = "Example"
"#;

#[test]
fn context() {
    let temp = tempfile::tempdir().expect("root");
    let root = temp.path();
    let tools = root.join("tools");
    std::fs::create_dir_all(&tools).expect("tools");
    let fixture = seat(root, &tools);
    fixture.seed();
    std::fs::write(
        root.join("plumb.toml"),
        format!("{SPEC}\n[release.depends]\noci = [\"source\"]\n"),
    )
    .expect("profile");
    std::fs::create_dir(root.join("source")).expect("source");
    for (path, text) in [
        ("Containerfile", "FROM scratch\nCOPY source /source\n"),
        ("source/tool", "original"),
        ("AGENTS.md", "operator"),
        (".dockerignore", "*"),
    ] {
        std::fs::write(root.join(path), text).expect("input");
        fixture.track(path);
    }
    std::fs::set_permissions(
        root.join("source/tool"),
        std::fs::Permissions::from_mode(0o755),
    )
    .expect("mode");
    symlink("tool", root.join("source/link")).expect("link");
    fixture.track("source");
    fixture.candidate();
    let captured = root.join("context.tar");
    executable(
        &tools.join("docker"),
        "#!/bin/sh\nset -eu\ntest \"$1\" = build\ncat > \"$PLUMB_TEST_CONTEXT\"\n",
    );
    std::fs::write(root.join("source/intruder"), "untracked").expect("intruder");
    std::fs::write(root.join("source/tool"), "unstaged").expect("working edit");
    let first = plan(&fixture);
    build(&fixture, &captured);
    let original = std::fs::read(&captured).expect("archive");
    let entries = entries(&captured);
    assert_eq!(entries.len(), 3);
    assert_eq!(entries["source/tool"], "original");
    assert_eq!(entries["source/link"], "link:tool");
    assert_eq!(
        entries["Containerfile"],
        "FROM scratch\nCOPY source /source\n"
    );
    std::fs::write(root.join("AGENTS.md"), "updated operator").expect("operator edit");
    fixture.track("AGENTS.md");
    let second = plan(&fixture);
    assert_eq!(first, second);
    build(&fixture, &captured);
    assert_eq!(original, std::fs::read(&captured).expect("archive"));
    fixture.track("source/tool");
    assert_ne!(first, plan(&fixture));
    build(&fixture, &captured);
    assert_ne!(original, std::fs::read(&captured).expect("archive"));
    for path in [
        "../outside",
        "/outside",
        ".",
        "source//tool",
        "source/",
        "*",
        "source/../tool",
    ] {
        std::fs::write(
            root.join("plumb.toml"),
            format!("{SPEC}\n[release.depends]\noci = [{path:?}]\n"),
        )
        .expect("invalid profile");
        let refused = fixture
            .command()
            .args(["ship", "surface"])
            .output()
            .expect("surface");
        let error = String::from_utf8_lossy(&refused.stderr);
        assert!(
            !refused.status.success() && error.contains("normalized repository path"),
            "{path}: {error}"
        );
    }
    std::fs::write(
        root.join("plumb.toml"),
        format!("{SPEC}\n[release.depends]\noci = [\"absent\"]\n"),
    )
    .expect("missing input");
    let refused = fixture
        .command()
        .args(["ship", "oci", "build"])
        .env("PLUMB_RELEASE_VERSION", "v1.0.0")
        .env("PLUMB_RELEASE_COMMIT", "c".repeat(40))
        .env("PLUMB_TEST_CONTEXT", &captured)
        .output()
        .expect("build");
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("no tracked files: absent"));
}

fn build(fixture: &Fixture<'_>, captured: &Path) {
    run(fixture
        .command()
        .args(["ship", "oci", "build"])
        .env("PLUMB_RELEASE_VERSION", "v1.0.0")
        .env("PLUMB_RELEASE_COMMIT", "c".repeat(40))
        .env("PLUMB_TEST_CONTEXT", captured));
}

fn plan(fixture: &Fixture<'_>) -> String {
    let surface = run(fixture.command().args(["ship", "surface"]));
    let surface: serde_json::Value = serde_json::from_slice(&surface.stdout).expect("surface");
    let roots = surface["publication"]["include"][0]["roots"]
        .as_array()
        .expect("roots");
    assert_eq!(
        roots,
        &vec![
            serde_json::json!("Containerfile"),
            serde_json::json!("source")
        ]
    );
    crate::identity::resources::fingerprint(fixture.root, serde_json::json!({"paths":roots}))
}

fn entries(path: &Path) -> BTreeMap<String, String> {
    let mut archive = tar::Archive::new(std::fs::File::open(path).expect("archive"));
    archive
        .entries()
        .expect("entries")
        .map(|entry| {
            let mut entry = entry.expect("entry");
            let path = entry
                .path()
                .expect("path")
                .to_str()
                .expect("UTF-8")
                .to_string();
            let mut bytes = String::new();
            if entry.header().entry_type().is_symlink() {
                bytes = format!(
                    "link:{}",
                    entry.link_name().expect("link").expect("target").display()
                );
            } else {
                entry.read_to_string(&mut bytes).expect("bytes");
            }
            if path == "source/tool" {
                assert_eq!(entry.header().mode().expect("mode"), 0o755);
            }
            assert_eq!(entry.header().mtime().expect("mtime"), 0);
            (path, bytes)
        })
        .collect()
}
