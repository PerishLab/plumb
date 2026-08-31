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
    let observed = root.join("docker.calls");
    let published = root.join("published");
    executable(
        &tools.join("docker"),
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> "$PLUMB_TEST_DOCKER"
case "$1" in
  login) cat >/dev/null ;;
  build|tag) ;;
  save)
    shift
    [ "$1" = --output ]
    printf 'exact image workload\n' > "$2"
    ;;
  load) printf 'Loaded image: held/probe:old\n' ;;
  pull) [ -f "$PLUMB_TEST_PUBLISHED" ] ;;
  push) touch "$PLUMB_TEST_PUBLISHED" ;;
  manifest) ;;
  image)
    case "$*" in
      *RepoDigests*) printf 'registry.example/owner/probe@sha256:%064d\n' 1 ;;
      *) printf '%s\n' "$PLUMB_TEST_PAYLOAD" ;;
    esac
    ;;
  *) exit 97 ;;
esac
"#,
    );
    executable(
        &tools.join("curl"),
        "#!/bin/sh\nset -eu\ncat \"$PLUMB_TEST_WORKLOAD\"\n",
    );
    let run = |version: &str, reuse: &str, workload: Option<&Path>| {
        let mut command = fixture.command();
        command
            .args(["ship", "oci", "exact", "--reuse", reuse])
            .env("PLUMB_RELEASE_VERSION", version)
            .env("PLUMB_RELEASE_COMMIT", "c".repeat(40))
            .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret")
            .env("PLUMB_TEST_DOCKER", &observed)
            .env("PLUMB_TEST_PUBLISHED", &published)
            .env("PLUMB_TEST_PAYLOAD", "c".repeat(40));
        if let Some(workload) = workload {
            command.env("PLUMB_TEST_WORKLOAD", workload);
        }
        super::super::world::run(&mut command)
    };

    let first = run("v1.0.0", r#"{"type":"none","source":""}"#, None);
    let first: serde_json::Value = serde_json::from_slice(&first.stdout).expect("image projection");
    assert_eq!(first["format"], "plumb.image-project/v1");
    assert_eq!(
        first["publication"],
        format!(
            "https://registry.example/v2/owner/probe/manifests/sha256:{}1",
            "0".repeat(63)
        )
    );
    let workload = Path::new(first["workload"].as_str().expect("workload path"));
    let carried = std::fs::read(workload).expect("image workload");

    std::fs::remove_file(&published).expect("reset publication");
    std::fs::write(&observed, "").expect("reset calls");
    let second = run(
        "v2.0.0",
        r#"{"type":"workload","source":"https://inventory.example/image.tar"}"#,
        Some(workload),
    );
    let second: serde_json::Value =
        serde_json::from_slice(&second.stdout).expect("reused image projection");
    assert_eq!(
        std::fs::read(second["workload"].as_str().expect("workload path"))
            .expect("reused workload"),
        carried
    );
    let calls = std::fs::read_to_string(&observed).expect("docker calls");
    assert!(calls.contains("load --input"), "{calls}");
    assert!(
        calls.contains("tag held/probe:old registry.example/owner/probe:v2.0.0"),
        "{calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("build ")),
        "{calls}"
    );
}
