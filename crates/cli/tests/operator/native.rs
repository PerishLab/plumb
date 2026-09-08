use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

fn run(command: &mut Command) -> String {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

#[test]
#[ignore = "explicit local Docker build and native regctl archive/authentication integration"]
fn authentication() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let native = run(Command::new("sh").args(["-c", "command -v regctl"]));
    let reference = format!(
        "127.0.0.1:1/plumb-proof/{}:fixture",
        root.file_name()
            .unwrap()
            .to_string_lossy()
            .to_ascii_lowercase()
            .replace('.', "")
    );
    assert!(
        !Command::new("docker")
            .args(["image", "inspect", &reference])
            .output()
            .unwrap()
            .status
            .success()
    );
    let image = Image(reference);
    std::fs::write(
        root.join("Containerfile"),
        "FROM scratch\nLABEL probe=authentication\n",
    )
    .unwrap();
    run(Command::new("docker")
        .args([
            "build",
            "--platform",
            "linux/amd64",
            "--label",
            &format!("org.opencontainers.image.revision={}", "c".repeat(40)),
            "--tag",
            &image.0,
            "--file",
        ])
        .arg(root.join("Containerfile"))
        .arg(root));
    let archive = root.join("image.tar");
    run(Command::new("docker")
        .args(["save", "--output"])
        .arg(&archive)
        .arg(&image.0));
    super::executable(
        &root.join("regctl"),
        r#"#!/bin/sh
set -eu
printf '%s\n' "$REGCTL_CONFIG" >> "$PLUMB_TEST_SEATS"
case "$1 $2" in
  'registry login') cat >/dev/null; exit 0 ;;
  'image copy')
    case "$3" in
      ocidir:*) printf '%s' "$3" > "$PLUMB_TEST_SOURCE"; exit 0 ;;
    esac
    ;;
  'image digest')
    case "$3" in
      127.0.0.1:1/*) exec "$PLUMB_TEST_NATIVE" image digest "$(cat "$PLUMB_TEST_SOURCE")" ;;
    esac
    ;;
esac
exec "$PLUMB_TEST_NATIVE" "$@"
"#,
    );
    for name in ["docker-credential-pass", "docker-credential-secretservice"] {
        super::executable(
            &root.join(name),
            "#!/bin/sh\ntouch \"$PLUMB_TEST_HELPER\"\nexit 1\n",
        );
    }
    let invoked = root.join("invoked");
    let seats = root.join("seats");
    let execution = super::execution(
        root,
        BTreeMap::from([
            (
                "PATH".into(),
                format!("{}:{}", root.display(), std::env::var("PATH").unwrap()),
            ),
            ("PLUMB_TEST_NATIVE".into(), native),
            (
                "PLUMB_TEST_SOURCE".into(),
                root.join("source").display().to_string(),
            ),
            ("PLUMB_TEST_SEATS".into(), seats.display().to_string()),
            ("PLUMB_TEST_HELPER".into(), invoked.display().to_string()),
        ]),
    );
    let spec = super::spec(root, "127.0.0.1:1", "plumb-proof/fixture", "Example");
    let error = {
        let image = super::super::Image::read(&spec, &archive, Some(&execution)).unwrap();
        image.publish(&spec, "Bearer fixture").unwrap_err()
    };
    assert!(error.contains("cannot be read anonymously"), "{error}");
    assert!(!invoked.exists(), "registry read invoked credential helper");
    let seats = std::fs::read_to_string(seats).unwrap();
    assert!(
        seats.lines().any(|path| path.ends_with("/anonymous")),
        "{seats}"
    );
    for path in seats.lines() {
        assert!(!Path::new(path).exists(), "configuration leaked: {path}");
    }
}

#[test]
#[ignore = "explicit loopback Forgejo retention fixture and native regctl; publishes only fixture images"]
fn publication() {
    let registry = std::env::var("PLUMB_TEST_REGISTRY").expect("loopback fixture registry");
    registry
        .strip_prefix("127.0.0.1:")
        .expect("loopback only")
        .parse::<u16>()
        .expect("fixture port");
    let archive = std::env::var("PLUMB_TEST_WORKLOAD").expect("native Docker-save fixture");
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let native = run(Command::new("sh").args(["-c", "command -v regctl"]));
    super::executable(
        &root.join("regctl"),
        r#"#!/bin/sh
exec "$PLUMB_TEST_NATIVE" --host "reg=$PLUMB_TEST_REGISTRY,tls=disabled" "$@"
"#,
    );
    let execution = super::execution(
        root,
        BTreeMap::from([
            (
                "PATH".into(),
                format!("{}:{}", root.display(), std::env::var("PATH").unwrap()),
            ),
            ("PLUMB_TEST_NATIVE".into(), native.clone()),
            ("PLUMB_TEST_REGISTRY".into(), registry.clone()),
        ]),
    );
    let repository = "retention-proof/plumb-archive";
    let spec = super::spec(root, &registry, repository, "retention-proof");
    let mut published = None;
    for _ in 0..2 {
        let image =
            super::super::Image::read(&spec, Path::new(&archive), Some(&execution)).unwrap();
        assert_eq!(image.provenance, "a".repeat(40));
        let current = image
            .publish(&spec, "Bearer Local-Fixture-Only-123")
            .unwrap();
        if let Some(prior) = &published {
            assert_eq!(&current, prior);
        }
        published = Some(current);
    }
    let tags = run(Command::new(native)
        .args([
            "--host",
            &format!("reg={registry},tls=disabled"),
            "tag",
            "ls",
            &format!("{registry}/{repository}"),
        ])
        .env("REGCTL_CONFIG", root.join("anonymous")));
    assert!(
        tags.lines().all(|line| line.starts_with("sha256-")),
        "{tags}"
    );
    assert_eq!(tags.lines().count(), 1, "{tags}");
}

struct Image(String);

impl Drop for Image {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["image", "rm", &self.0])
            .output();
    }
}
