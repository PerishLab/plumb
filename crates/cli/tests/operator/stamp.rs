use std::os::unix::fs::PermissionsExt;

const STAMPED: &str = r#"#!/bin/sh
set -eu
if [ "$1" = metadata ]; then printf '%s\n' "{\"packages\":[{\"name\":\"family-core\",\"version\":\"1.2.0\",\"manifest_path\":\"$PWD/crates/core/Cargo.toml\",\"targets\":[]},{\"name\":\"family-macro\",\"version\":\"1.2.0\",\"manifest_path\":\"$PWD/crates/macro/Cargo.toml\",\"targets\":[]}],\"target_directory\":\"$PWD/target\"}"; exit 0; fi
grep -F 'version = "1.2.0-beta.8"' Cargo.toml >/dev/null
grep -F 'version = "=1.2.0-beta.8"' crates/core/Cargo.toml >/dev/null
grep -F 'version.workspace = true' crates/macro/Cargo.toml >/dev/null
name=""
prev=""
for arg in "$@"; do
  if [ "$prev" = --package ]; then name="$arg"; fi
  prev="$arg"
done
mkdir -p "target/package/$name-1.2.0-beta.8"
printf 'version = "1.2.0-beta.8"\n' > "target/package/$name-1.2.0-beta.8/Cargo.toml"
tar -czf "target/package/$name-1.2.0-beta.8.crate" -C target/package "$name-1.2.0-beta.8"
printf '%s\n' "$name" >> projected
"#;

#[test]
fn stamped() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    let tools = root.join("tools");
    let artifacts = root.join("artifacts");
    std::fs::create_dir_all(&tools).expect("tool root");
    std::fs::create_dir_all(&artifacts).expect("artifact root");
    std::fs::create_dir_all(root.join("crates/core")).expect("crate root");
    std::fs::create_dir_all(root.join("crates/macro")).expect("crate root");
    let fixture = super::fixture::Fixture {
        root,
        tools: &tools,
    };
    fixture.seed();
    let cargo = tools.join("cargo");
    std::fs::write(&cargo, STAMPED).expect("fake cargo");
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).expect("cargo mode");
    for (seat, text) in [
        (
            "plumb.toml",
            &format!(
                "{}[release.cargo]\nregistry = \"perish\"\npackages = [\"family-macro\", \"family-core\"]\n",
                super::fixture::SPEC
            ),
        ),
        (
            "Cargo.toml",
            &"[workspace]\nmembers = [\"crates/core\", \"crates/macro\"]\nresolver = \"3\"\n\n[workspace.package]\nversion = \"1.2.0\"\nedition = \"2024\"\n".to_string(),
        ),
        (
            "crates/macro/Cargo.toml",
            &"[package]\nname = \"family-macro\"\nversion.workspace = true\n".to_string(),
        ),
        (
            "crates/core/Cargo.toml",
            &"[package]\nname = \"family-core\"\nversion.workspace = true\n\n[dependencies.family-macro]\npath = \"../macro\"\nversion = \"=1.2.0\"\n".to_string(),
        ),
    ] {
        std::fs::write(root.join(seat), text).expect("fixture file");
    }
    fixture.changelog("v1.2.0");
    fixture.track("Cargo.toml");
    fixture.track("crates");
    let candidate = fixture.candidate();
    fixture.tag("v1.2.0-beta.7");
    fixture.archive(&artifacts, "v1.2.0-beta.7");
    let out = root.join("beta");
    super::fixture::run(
        fixture
            .command()
            .args(["release", "compile"])
            .env("PLUMB_RELEASE_CHANNEL", "beta")
            .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.7")
            .env("PLUMB_RELEASE_COMMIT", &candidate)
            .env("PLUMB_RELEASE_ARTIFACTS", &artifacts)
            .env("PLUMB_RELEASE_OUTPUT", &out),
    );
    let published = root.join("releases/v1");
    std::fs::create_dir_all(published.join("channels")).expect("published root");
    std::fs::write(
        published.join("channels/stable.json"),
        format!(
            concat!(
                r#"{{"schema":1,"product":"probe","channel":"stable","releaseVersion":"v1.1.0","#,
                r#""commit":"{}","managers":{{}},"seal":{{"name":"seal.json","#,
                r#""mime":"application/json","sha256":"{}","size":3,"#,
                r#""url":"https://releases.test/v1/seal.json"}}}}"#
            ),
            candidate,
            "0".repeat(64)
        ),
    )
    .expect("stable pointer");
    std::fs::copy(out.join("seal.json"), published.join("seal.json")).expect("published seal");
    std::fs::create_dir_all(root.join("crates/core/src")).expect("source root");
    std::fs::write(
        root.join("crates/core/src/lib.rs"),
        "pub const HELD: u8 = 1;\n",
    )
    .expect("moved source");
    fixture.track("crates");

    let said = String::from_utf8_lossy(
        &super::fixture::run(
            fixture
                .command()
                .args(["ship", "cargo", "rehearse"])
                .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.8")
                .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret"),
        )
        .stdout,
    )
    .to_string();
    assert!(
        said.contains("rehearsed Cargo attachment for v1.2.0-beta.8"),
        "{said}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("projected")).expect("projection"),
        "family-macro\n",
        "a package requiring a sibling cannot be verified before that sibling is published"
    );
    assert!(
        said.contains("family-core requires a sibling this release has not published yet"),
        "{said}"
    );
    assert!(!said.contains("not projected"), "{said}");

    super::fixture::run(
        fixture
            .command()
            .current_dir(root)
            .args(["release", "compile"])
            .env("PLUMB_RELEASE_ROOT", ".")
            .env("PLUMB_RELEASE_CHANNEL", "beta")
            .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.9")
            .env("PLUMB_RELEASE_COMMIT", &candidate)
            .env("PLUMB_RELEASE_ARTIFACTS", &artifacts)
            .env("PLUMB_RELEASE_OUTPUT", root.join("relative")),
    );
}
