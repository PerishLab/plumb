pub(super) const CLIENT: &str = r#"#!/bin/sh
set -eu
scenario=$(cat "$PLUMB_TEST_SCENARIO")
[ -z "${DOCKER_AUTH_CONFIG+x}" ]
printf 'seat %s\n' "$REGCTL_CONFIG" >> "$PLUMB_TEST_CALLS"
printf 'regctl %s\n' "$*" >> "$PLUMB_TEST_CALLS"
case "$1 $2" in
  'config set')
    [ "$3" = --docker-cred=false ]
    [ "$4" = --docker-cert=false ]
    printf '{}\n' > "$REGCTL_CONFIG"
    ;;
  'registry login')
    [ "${REGCTL_CONFIG##*/}" = authenticated ]
    [ "$3" = registry.example ]
    [ "$4" = --user ]; [ "$5" = Example ]; [ "$6" = --pass-stdin ]
    [ "$(cat)" = secret ]
    [ "$scenario" != login ]
    printf 'authenticated\n' > "$REGCTL_CONFIG"
    ;;
  'image import') ;;
  'image config')
    if [ "$scenario" = provenance ]; then payload=unproved; else payload=cccccccccccccccccccccccccccccccccccccccc; fi
    printf '{"config":{"Labels":{"org.opencontainers.image.revision":"%s"}}}\n' "$payload"
    ;;
  'image digest')
    if [ "$scenario" = invalid ]; then printf 'invalid\n'; exit; fi
    case "$3" in
      registry.example/*)
        if [ "$scenario" = drift ]; then printf 'sha256:%064d\n' 2; exit; fi
        ;;
      *readback*)
        if [ "$scenario" = readback ]; then printf 'sha256:%064d\n' 2; exit; fi
        ;;
    esac
    printf 'sha256:%064d\n' 1
    ;;
  'image copy')
    case "$3" in
      ocidir:*) [ "$(cat "$REGCTL_CONFIG")" = authenticated ] ;;
      registry.example/*)
        [ "${REGCTL_CONFIG##*/}" = anonymous ]
        [ "$(cat "$REGCTL_CONFIG")" = '{}' ]
        [ "$5" = --force-recursive ]
        [ "$scenario" != anonymous ]
        ;;
      *) exit 98 ;;
    esac
    ;;
  *) exit 97 ;;
esac
"#;

#[test]
#[ignore = "explicit loopback Forgejo retention fixture and native regctl; publishes only fixture images"]
fn publication() {
    use super::super::world::{Fixture, run};
    use std::process::Command;

    let registry = std::env::var("PLUMB_TEST_REGISTRY").expect("loopback fixture registry");
    let port = registry.strip_prefix("127.0.0.1:").expect("loopback only");
    port.parse::<u16>().expect("fixture port");
    let temp = tempfile::tempdir().expect("root");
    let root = temp.path();
    let tools = root.join("tools");
    std::fs::create_dir(&tools).expect("tools");
    let fixture = Fixture {
        root,
        tools: &tools,
    };
    fixture.seed();
    let native = run(Command::new("sh").args(["-c", "command -v regctl"]));
    let native = String::from_utf8(native.stdout).expect("regctl path");
    let repository = "retention-proof/plumb-archive";
    std::fs::write(root.join("plumb.toml"), format!(
        "[release]\nproduct = \"probe\"\nauthority = \"https://releases.test\"\n[release.oci]\nregistry = \"{registry}\"\nimage = \"{repository}\"\naccount = \"retention-proof\"\n"
    )).expect("profile");
    super::executable(&tools.join("docker"), "#!/bin/sh\nexit 97\n");
    super::executable(
        &tools.join("curl"),
        "#!/bin/sh\ncat \"$PLUMB_TEST_WORKLOAD\"\n",
    );
    super::executable(
        &tools.join("regctl"),
        r#"#!/bin/sh
exec "$PLUMB_TEST_NATIVE" --host "reg=$PLUMB_TEST_REGISTRY,tls=disabled" "$@"
"#,
    );
    let mut published = None;
    for version in ["v1.0.0", "v2.0.0"] {
        let output = run(fixture
            .command()
            .args([
                "ship",
                "oci",
                "exact",
                "--reuse",
                r#"{"type":"workload","source":"https://fixture.invalid/workload.tar"}"#,
            ])
            .env("PLUMB_RELEASE_VERSION", version)
            .env("PLUMB_RELEASE_COMMIT", "d".repeat(40))
            .env(
                "PLUMB_RELEASE_REGISTRY_TOKEN",
                "Bearer Local-Fixture-Only-123",
            )
            .env("PLUMB_TEST_NATIVE", native.trim()));
        let body: serde_json::Value = serde_json::from_slice(&output.stdout).expect("projection");
        assert_eq!(body["provenance"], "a".repeat(40));
        assert_eq!(body["version"], version);
        if let Some(prior) = published {
            assert_eq!(body["publication"], prior);
        }
        published = Some(body["publication"].clone());
    }
    let tags = run(Command::new(native.trim())
        .args([
            "--host",
            &format!("reg={registry},tls=disabled"),
            "tag",
            "ls",
            &format!("{registry}/{repository}"),
        ])
        .env("REGCTL_CONFIG", root.join("anonymous")));
    let tags = String::from_utf8(tags.stdout).expect("tags");
    assert!(
        tags.lines().all(|line| line.starts_with("sha256-")),
        "{tags}"
    );
    assert_eq!(tags.lines().count(), 1, "{tags}");
}
