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
    let scenario = root.join("scenario");
    let pulled = root.join("pulled");
    std::fs::write(&scenario, "normal").expect("scenario");
    executable(
        &tools.join("docker"),
        r#"#!/bin/sh
set -eu
scenario=$(cat "$PLUMB_TEST_SCENARIO")
printf '%s\n' "$*" >> "$PLUMB_TEST_DOCKER"
case "$1" in
  login) cat >/dev/null ;;
  build|tag) ;;
  save)
    shift
    [ "$1" = --output ]
    printf 'exact image workload\n' > "$2"
    ;;
  load)
    printf 'Loaded image: held/probe:old\n'
    if [ "$scenario" = multiple ]; then printf 'Loaded image: extra/probe:old\n'; fi
    ;;
  pull) [ -f "$PLUMB_TEST_PUBLISHED" ] && touch "$PLUMB_TEST_PULLED" ;;
  push) touch "$PLUMB_TEST_PUBLISHED" ;;
  manifest) ;;
  image)
    case "$*" in
      *RepoDigests*)
        case "$scenario" in
          absent) printf '["held/probe@sha256:%064d"]\n' 2 ;;
          duplicate) printf '["registry.example/owner/probe@sha256:%064d","registry.example/owner/probe@sha256:%064d"]\n' 1 2 ;;
          invalid) printf '["registry.example/owner/probe@sha256:invalid"]\n' ;;
          *) printf '["held/probe@sha256:%064d","registry.example/owner/probe@sha256:%064d"]\n' 2 1 ;;
        esac
        ;;
      *'{{.Id}}'*)
        if [ "$scenario" = drift ] && [ -f "$PLUMB_TEST_PULLED" ]; then
          printf 'sha256:%064d\n' 4
        else
          printf 'sha256:%064d\n' 3
        fi
        ;;
      *)
        if [ "$scenario" = provenance ]; then printf 'unproved\n'; else printf '%s\n' "$PLUMB_TEST_PAYLOAD"; fi
        ;;
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
    let run = |version: &str, commit: &str, reuse: &str, workload: Option<&Path>| {
        let mut command = fixture.command();
        command
            .args(["ship", "oci", "exact", "--reuse", reuse])
            .env("PLUMB_RELEASE_VERSION", version)
            .env("PLUMB_RELEASE_COMMIT", commit)
            .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret")
            .env("PLUMB_TEST_DOCKER", &observed)
            .env("PLUMB_TEST_PUBLISHED", &published)
            .env("PLUMB_TEST_SCENARIO", &scenario)
            .env("PLUMB_TEST_PULLED", &pulled)
            .env("PLUMB_TEST_PAYLOAD", "c".repeat(40));
        if let Some(workload) = workload {
            command.env("PLUMB_TEST_WORKLOAD", workload);
        }
        command.output().expect("plumb should run")
    };

    let first = run(
        "v1.0.0",
        &"c".repeat(40),
        r#"{"type":"none","source":""}"#,
        None,
    );
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first: serde_json::Value = serde_json::from_slice(&first.stdout).expect("image projection");
    assert_eq!(first["format"], "plumb.image-project/v1");
    assert_eq!(first["provenance"], "c".repeat(40));
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
        &"d".repeat(40),
        r#"{"type":"workload","source":"https://inventory.example/image.tar"}"#,
        Some(workload),
    );
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second: serde_json::Value =
        serde_json::from_slice(&second.stdout).expect("reused image projection");
    assert_eq!(second["version"], "v2.0.0");
    assert_eq!(second["provenance"], first["provenance"]);
    assert_eq!(second["publication"], first["publication"]);
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
    for (case, expected) in [
        ("multiple", "exactly one image"),
        ("provenance", "no valid org.opencontainers.image.revision"),
        ("drift", "published image drift"),
        ("absent", "no digest for its publication repository"),
        ("duplicate", "no unique valid publication digest"),
        ("invalid", "no unique valid publication digest"),
    ] {
        std::fs::write(&scenario, case).expect("scenario");
        std::fs::write(&observed, "").expect("reset calls");
        if pulled.exists() {
            std::fs::remove_file(&pulled).expect("reset readback");
        }
        let refused = run(
            "v2.0.0",
            &"d".repeat(40),
            r#"{"type":"workload","source":"https://inventory.example/image.tar"}"#,
            Some(workload),
        );
        let error = String::from_utf8_lossy(&refused.stderr);
        assert!(
            !refused.status.success() && error.contains(expected),
            "{case}: {error}"
        );
        let calls = std::fs::read_to_string(&observed).expect("docker calls");
        assert!(
            !calls.lines().any(|line| line.starts_with("push ")),
            "{case}: {calls}"
        );
    }
}
