use super::super::world::Fixture;
use std::path::Path;
use std::process::Output;

fn promote(fixture: &Fixture<'_>, commit: &str, proof: &Path, artifacts: &Path) -> Output {
    fixture
        .command()
        .args(["release", "promote"])
        .env("PLUMB_RELEASE_COMMIT", commit)
        .env("PLUMB_RELEASE_VERSION", "v1.2.0")
        .env("PLUMB_RELEASE_PROMOTION", proof)
        .env("PLUMB_RELEASE_ARTIFACTS", artifacts)
        .output()
        .expect("plumb should run")
}

#[test]
fn derived() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    let tools = root.join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root,
        tools: &tools,
    };
    fixture.seed();
    let candidate = fixture.candidate();
    let proof = root.join("promotion/seal.json");
    let artifacts = root.join("promotion/artifacts");

    let bare = promote(&fixture, &candidate, &proof, &artifacts);
    let absent = String::from_utf8_lossy(&bare.stderr).to_string();
    assert!(!bare.status.success(), "{absent}");
    assert!(absent.contains("no published exact seal"), "{absent}");

    for exact in ["v1.2.0-beta.7", "v1.2.0-beta.8"] {
        fixture.tag(exact);
        fixture.seal(exact);
    }
    carried(root, "v1.2.0-beta.8");
    let many = promote(&fixture, &candidate, &proof, &artifacts);
    let latest = format!(
        "{}{}",
        String::from_utf8_lossy(&many.stdout),
        String::from_utf8_lossy(&many.stderr)
    );
    assert!(many.status.success(), "{latest}");
    assert!(latest.contains("v1.2.0-beta.8"), "{latest}");
    assert!(proof.exists(), "{latest}");
    assert_eq!(
        std::fs::read(artifacts.join("probe-x86_64-unknown-linux-gnu.tar.gz"))
            .expect("promoted artifact"),
        b"promoted binary"
    );

    std::fs::remove_file(&proof).expect("proof should clear");
    std::fs::remove_dir_all(&artifacts).expect("artifacts should clear");
    for exact in ["v1.2.0-beta.9", "v1.2.0-beta.10"] {
        fixture.tag(exact);
        fixture.seal(exact);
    }
    carried(root, "v1.2.0-beta.10");
    let ranked = promote(&fixture, &candidate, &proof, &artifacts);
    let shown = format!(
        "{}{}",
        String::from_utf8_lossy(&ranked.stdout),
        String::from_utf8_lossy(&ranked.stderr)
    );
    assert!(ranked.status.success(), "{shown}");
    assert!(shown.contains("v1.2.0-beta.10"), "{shown}");
}

#[test]
fn drift() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    let tools = root.join("tools");
    std::fs::create_dir(&tools).expect("tool root");
    let fixture = Fixture {
        root,
        tools: &tools,
    };
    fixture.seed();
    let candidate = fixture.candidate();
    fixture.tag("v1.2.0-beta.1");
    fixture.seal("v1.2.0-beta.1");
    let object = carried(root, "v1.2.0-beta.1");
    std::fs::write(object, b"drifted binary").expect("drift object");
    let proof = root.join("promotion/seal.json");
    let artifacts = root.join("promotion/artifacts");

    let output = promote(&fixture, &candidate, &proof, &artifacts);
    let shown = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{shown}");
    assert!(shown.contains("disagrees with its proof"), "{shown}");
    assert!(!proof.exists(), "a failed promotion must be retryable");
    assert!(
        !artifacts.exists(),
        "partial artifacts must not become visible"
    );
}

fn carried(root: &Path, version: &str) -> std::path::PathBuf {
    use sha2::{Digest, Sha256};

    let body = b"promoted binary";
    let digest = Sha256::digest(body)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let object = root
        .join("releases/v1/objects/sha256")
        .join(&digest)
        .join("probe-x86_64-unknown-linux-gnu.tar.gz");
    std::fs::create_dir_all(object.parent().expect("object parent")).expect("object root");
    std::fs::write(&object, body).expect("object");
    let seal = root
        .join("releases/v1/releases/beta")
        .join(version)
        .join("seal.json");
    let mut held: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&seal).expect("seal text"))
            .expect("seal json");
    held["artifacts"]["linux-x64"] = serde_json::json!({
        "name": "probe-x86_64-unknown-linux-gnu.tar.gz",
        "mime": "application/gzip",
        "sha256": &digest,
        "size": body.len(),
        "url": format!(
            "https://releases.test/v1/objects/sha256/{digest}/probe-x86_64-unknown-linux-gnu.tar.gz"
        ),
    });
    std::fs::write(
        seal,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&held).expect("seal encode")
        ),
    )
    .expect("seal update");
    object
}
