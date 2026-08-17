use super::fixture::Fixture;
use std::path::Path;
use std::process::Output;

fn promote(fixture: &Fixture<'_>, commit: &str, proof: &Path) -> Output {
    fixture
        .command()
        .args(["release", "promote"])
        .env("PLUMB_RELEASE_COMMIT", commit)
        .env("PLUMB_RELEASE_VERSION", "v1.2.0")
        .env("PLUMB_RELEASE_PROMOTION", proof)
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
    fixture.changelog("v1.2.0");
    let candidate = fixture.candidate();
    let proof = root.join("promotion/seal.json");

    let bare = promote(&fixture, &candidate, &proof);
    let absent = String::from_utf8_lossy(&bare.stderr).to_string();
    assert!(!bare.status.success(), "{absent}");
    assert!(absent.contains("no published exact seal"), "{absent}");

    for exact in ["v1.2.0-beta.7", "v1.2.0-beta.8"] {
        fixture.tag(exact);
        fixture.seal(exact);
    }
    let many = promote(&fixture, &candidate, &proof);
    let crowded = String::from_utf8_lossy(&many.stderr).to_string();
    assert!(!many.status.success(), "{crowded}");
    assert!(crowded.contains("2 published exact seals"), "{crowded}");
    assert!(
        crowded.contains("v1.2.0-beta.7, v1.2.0-beta.8"),
        "{crowded}"
    );
    assert!(!proof.exists(), "{crowded}");
}
