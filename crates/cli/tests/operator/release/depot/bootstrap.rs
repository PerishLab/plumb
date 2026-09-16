use super::run;
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

pub(super) fn validator(root: &std::path::Path) {
    let stage = root.join("validator");
    std::fs::create_dir(&stage).expect("validator stage");
    let binary = stage.join("probe");
    std::fs::write(
        &binary,
        "#!/bin/sh\n[ \"$1\" = --version ] && { echo 'probe v1.2.0-beta.1'; exit; }\n[ -z \"$PLUMB_GUARD_CONFIGURATION\" ] || exit 9\nif [ -f \"$PROBE_DEPOT_SNAPSHOT/rules/products.toml\" ]; then\n[ \"$(cat ectropy.toml)\" = '[comment]' ] || exit 7\n[ \"$PLUMB_GUARD_VIEW\" = \"$PWD\" ] || exit 8\ngrep -q '^authority = \"' .plumb/affirmed.toml || exit 10\nfi\n[ \"$1\" = doctor ] && [ -f \"$PROBE_DEPOT_SNAPSHOT/manifest.toml\" ] && [ -f \"$PROBE_DEPOT_SNAPSHOT/rules/probe.toml\" ]\n",
    )
    .expect("validator");
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).expect("mode");
    let archive = root.join("probe-x86_64-unknown-linux-gnu.tar.gz");
    run(Command::new("tar").args([
        "-C",
        stage.to_str().expect("stage"),
        "-czf",
        archive.to_str().expect("archive"),
        "probe",
    ]));
    let bytes = std::fs::read(&archive).expect("archive bytes");
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let name = archive
        .file_name()
        .and_then(|held| held.to_str())
        .expect("name");
    let object = root
        .join("releases/v1/objects/sha256")
        .join(&digest)
        .join(name);
    std::fs::create_dir_all(object.parent().expect("object parent")).expect("object root");
    std::fs::write(&object, &bytes).expect("published validator");
    let seal = root.join("releases/v1/releases/beta/v1.2.0-beta.1/seal.json");
    let mut held: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&seal).expect("seal")).expect("seal json");
    held["artifacts"]["linux-x64"] = serde_json::json!({
        "name": name,
        "mime": "application/gzip",
        "sha256": digest,
        "size": bytes.len(),
        "url": format!("https://releases.test/v1/objects/sha256/{digest}/{name}"),
    });
    std::fs::write(seal, serde_json::to_vec_pretty(&held).expect("seal body")).expect("seal write");
}

pub(super) fn documents(root: &std::path::Path) {
    std::fs::create_dir_all(root.join(".plumb")).unwrap();
    std::fs::write(root.join("AGENTS.md"), "# Reviewed operations\n").unwrap();
    std::fs::write(root.join(".plumb/affirmed.toml"), "# frozen confirmation\n").unwrap();
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["add", "AGENTS.md", ".plumb/affirmed.toml"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn explicit() {
    let root = tempfile::tempdir().unwrap();
    let generation = "a".repeat(64);
    for (arguments, expected) in [
        (
            vec!["guard", "--generation", &generation],
            "--configuration-marker",
        ),
        (
            vec!["guard", "--configuration-marker", "v1.2.3"],
            "--generation",
        ),
        (
            vec![
                "guard",
                "--configuration-marker",
                "v1.2.3",
                "--generation",
                &generation,
            ],
            "explicit isolated PLUMB_HOME",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .current_dir(root.path())
            .env_remove("PLUMB_HOME")
            .args(arguments)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn selection() {
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let version = plumb::version!("PLUMB").to_string();
    let binding = crate::marker::prepare(
        root.path(),
        home.path(),
        "[release]\nproduct='probe'\nauthority='https://release.test'\n",
        &version,
    );
    for (marker, generation) in [
        ("v99.0.0", binding["configuration"].as_str().unwrap()),
        (
            version.as_str(),
            "0000000000000000000000000000000000000000000000000000000000000000",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .current_dir(root.path())
            .env("PLUMB_HOME", home.path())
            .args([
                "guard",
                "--configuration-marker",
                marker,
                "--generation",
                generation,
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("does not match explicit marker and generation")
        );
    }
}
