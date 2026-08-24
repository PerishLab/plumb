use super::world::{Fixture, SPEC};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;

const DOCKER: &str = "#!/bin/sh\nexit 0\n";
const ARCHIVE: &str = "probe-x86_64-unknown-linux-gnu.tar.gz";

fn seat<'a>(root: &'a std::path::Path, tools: &'a std::path::Path) -> Fixture<'a> {
    Fixture { root, tools }
}

fn build(fixture: &Fixture<'_>, artifacts: &std::path::Path) -> std::process::Output {
    fixture
        .command()
        .args(["ship", "oci", "build"])
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.7")
        .env("PLUMB_RELEASE_COMMIT", "c".repeat(40))
        .env("PLUMB_RELEASE_ARTIFACTS", artifacts)
        .output()
        .expect("plumb should run")
}

#[test]
fn payload() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    let tools = root.join("tools");
    let artifacts = root.join("artifacts");
    let empty = root.join("empty");
    for held in [&tools, &artifacts, &empty] {
        std::fs::create_dir_all(held).expect("fixture root");
    }
    let fixture = seat(root, &tools);
    fixture.seed();
    std::fs::write(
        root.join("plumb.toml"),
        format!(
            "{SPEC}[release.oci]\nregistry = \"example.invalid\"\nimage = \"owner/probe\"\naccount = \"Example\"\n"
        ),
    )
    .expect("manifest");
    std::fs::write(root.join("Containerfile"), "FROM scratch\n").expect("container file");
    let docker = tools.join("docker");
    std::fs::write(&docker, DOCKER).expect("fake docker");
    std::fs::set_permissions(&docker, std::fs::Permissions::from_mode(0o755)).expect("docker mode");
    fixture.archive(&artifacts, "v1.2.0-beta.7");

    let local = build(&fixture, &artifacts);
    let said = String::from_utf8_lossy(&local.stdout).to_string();
    let bytes = std::fs::read(artifacts.join(ARCHIVE)).expect("archive");
    let digest = format!("{:x}", Sha256::digest(&bytes));
    assert!(
        local.status.success() && said.contains(&digest),
        "a local archive is the payload it labels: {said}"
    );

    let object = root.join("releases/v1/objects/sha256").join(&digest);
    std::fs::create_dir_all(object.parent().expect("object root")).expect("object root");
    std::fs::write(&object, &bytes).expect("published object");
    let seal = root.join("releases/v1/releases/beta/v1.2.0-beta.7");
    std::fs::create_dir_all(&seal).expect("seal root");
    let record = |sha: &str| {
        format!(
            concat!(
                r#"{{"artifacts":{{"linux-x64":{{"name":"{}","mime":"application/gzip","#,
                r#""sha256":"{}","size":{},"#,
                r#""url":"https://releases.test/v1/objects/sha256/{}"}}}}}}"#
            ),
            ARCHIVE,
            sha,
            bytes.len(),
            digest
        )
    };
    std::fs::write(seal.join("seal.json"), record(&digest)).expect("published seal");

    let published = build(&fixture, &empty);
    let said = String::from_utf8_lossy(&published.stdout).to_string();
    assert!(
        published.status.success() && said.contains(&digest),
        "an absent archive is taken from the authority that published it: {said}{}",
        String::from_utf8_lossy(&published.stderr)
    );

    std::fs::write(seal.join("seal.json"), record(&"0".repeat(64))).expect("published seal");
    let drifted = build(&fixture, &empty);
    let refused = String::from_utf8_lossy(&drifted.stderr).to_string();
    assert!(
        !drifted.status.success() && refused.contains("published payload drift"),
        "a payload the seal does not name refuses: {refused}"
    );
}
