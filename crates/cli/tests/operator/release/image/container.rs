use super::{executable, seat};
use std::path::Path;

#[test]
fn exact() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    let tools = root.join("tools");
    std::fs::create_dir_all(&tools).expect("tool seat");
    let fixture = seat(root, &tools);
    fixture.seed();
    std::fs::write(
        root.join("plumb.toml"),
        concat!(
            "[release]\nproduct = \"probe\"\nauthority = \"https://releases.test\"\n",
            "[release.oci]\nregistry = \"registry.example\"\n",
            "image = \"owner/probe\"\naccount = \"Example\"\n",
        ),
    )
    .expect("manifest");
    std::fs::write(root.join("Containerfile"), "FROM scratch\n").expect("container file");
    fixture.track("Containerfile");
    let observed = root.join("calls");
    let scenario = root.join("scenario");
    let source = root.join("source.tar");
    archive(&source, 1);
    std::fs::write(&observed, "").expect("calls");
    std::fs::write(&scenario, "normal").expect("scenario");
    executable(
        &tools.join("docker"),
        r#"#!/bin/sh
set -eu
printf 'docker %s\n' "$*" >> "$PLUMB_TEST_CALLS"
case "$1" in
  build) cat >/dev/null ;;
  save) [ "$2" = --output ]; cp "$PLUMB_TEST_WORKLOAD" "$3" ;;
  *) exit 97 ;;
esac
"#,
    );
    executable(&tools.join("regctl"), super::registry::CLIENT);
    executable(
        &tools.join("curl"),
        "#!/bin/sh\ncat \"$PLUMB_TEST_WORKLOAD\"\n",
    );
    let ambient = root.join("ambient");
    std::fs::write(&ambient, "untouched").expect("ambient credential");
    let run = |version: &str, commit: &str, reuse: &str| {
        let output = fixture
            .command()
            .args(["ship", "oci", "exact", "--reuse", reuse])
            .env("PLUMB_RELEASE_VERSION", version)
            .env("PLUMB_RELEASE_COMMIT", commit)
            .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret")
            .env("REGCTL_CONFIG", &ambient)
            .env("DOCKER_AUTH_CONFIG", "ambient credentials")
            .env("PLUMB_TEST_CALLS", &observed)
            .env("PLUMB_TEST_SCENARIO", &scenario)
            .env("PLUMB_TEST_WORKLOAD", &source)
            .output()
            .expect("plumb");
        let calls = std::fs::read_to_string(&observed).expect("calls");
        assert!(!calls.contains("secret"), "credential entered argv");
        for path in calls.lines().filter_map(|line| line.strip_prefix("seat ")) {
            assert!(!Path::new(path).exists(), "configuration leaked: {path}");
        }
        assert_eq!(
            std::fs::read_to_string(&ambient).expect("ambient"),
            "untouched"
        );
        output
    };
    let first = run("v1.0.0", &"c".repeat(40), r#"{"type":"none","source":""}"#);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first: serde_json::Value = serde_json::from_slice(&first.stdout).expect("projection");
    assert_eq!(first["format"], "plumb.image-project/v1");
    assert_eq!(first["provenance"], "c".repeat(40));
    assert_eq!(
        first["publication"],
        format!(
            "https://registry.example/v2/owner/probe/manifests/sha256:{}1",
            "0".repeat(63)
        )
    );
    let calls = std::fs::read_to_string(&observed).expect("calls");
    assert!(calls.contains("docker build "), "{calls}");
    assert!(calls.contains("docker save "), "{calls}");
    std::fs::write(&observed, "").expect("reset calls");
    let reuse = r#"{"type":"workload","source":"https://inventory.example/image.tar"}"#;
    let second = run("v2.0.0", &"d".repeat(40), reuse);
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second: serde_json::Value = serde_json::from_slice(&second.stdout).expect("projection");
    assert_eq!(second["version"], "v2.0.0");
    assert_eq!(second["provenance"], first["provenance"]);
    assert_eq!(second["publication"], first["publication"]);
    assert_eq!(
        std::fs::read(second["workload"].as_str().expect("path")).expect("workload"),
        std::fs::read(&source).expect("source")
    );
    let calls = std::fs::read_to_string(&observed).expect("calls");
    assert!(!calls.contains("docker "), "{calls}");
    assert!(!calls.contains(":v2.0.0"), "{calls}");
    assert!(
        calls.contains("registry.example/owner/probe:sha256-"),
        "{calls}"
    );
    assert!(calls.contains("--force-recursive"), "{calls}");
    for (case, expected) in [
        ("invalid", "no valid manifest digest"),
        ("provenance", "no valid org.opencontainers.image.revision"),
        ("login", "image attachment login failed"),
        ("drift", "published image drift"),
        ("anonymous", "cannot be read anonymously"),
        ("readback", "readback changed content identity"),
    ] {
        std::fs::write(&scenario, case).expect("scenario");
        std::fs::write(&observed, "").expect("reset calls");
        let refused = run("v2.0.0", &"d".repeat(40), reuse);
        let error = String::from_utf8_lossy(&refused.stderr);
        assert!(
            !refused.status.success() && error.contains(expected),
            "{case}: {error}"
        );
        assert!(
            !std::fs::read_to_string(&observed)
                .expect("calls")
                .contains("docker ")
        );
    }
    archive(&source, 2);
    let refused = run("v2.0.0", &"d".repeat(40), reuse);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("exactly one image"));
}

fn archive(path: &Path, count: usize) {
    let file = std::fs::File::create(path).expect("archive");
    let mut archive = tar::Builder::new(file);
    let body = serde_json::to_vec(&vec![serde_json::json!({"Config":"config.json"}); count])
        .expect("manifest");
    let mut header = tar::Header::new_gnu();
    header.set_mode(0o644);
    header.set_size(body.len() as u64);
    header.set_cksum();
    archive
        .append_data(&mut header, "manifest.json", body.as_slice())
        .expect("manifest");
    archive.finish().expect("archive");
}
