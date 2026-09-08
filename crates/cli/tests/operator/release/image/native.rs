use super::super::world::{Fixture, run};
use std::process::Command;

#[test]
#[ignore = "explicit integration with a local Docker daemon; builds and removes one owned scratch image"]
fn context() {
    let temp = tempfile::tempdir().expect("root");
    let root = temp.path();
    let tools = root.join("tools");
    std::fs::create_dir(&tools).expect("tools");
    let fixture = Fixture {
        root,
        tools: &tools,
    };
    fixture.seed();
    let name = root
        .file_name()
        .expect("name")
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace('.', "");
    let reference = format!("registry.invalid/plumb-proof/{name}:v1.0.0");
    assert!(
        !Command::new("docker")
            .args(["image", "inspect", &reference])
            .output()
            .expect("Docker")
            .status
            .success(),
        "test image already exists"
    );
    let image = Image {
        fixture: &fixture,
        reference,
    };
    std::fs::write(root.join("plumb.toml"), format!(
        "[release]\nproduct = \"probe\"\nauthority = \"https://releases.test\"\n[release.oci]\nregistry = \"registry.invalid\"\nimage = \"plumb-proof/{name}\"\naccount = \"Example\"\n[release.depends]\noci = [\"source\"]\n"
    )).expect("profile");
    std::fs::create_dir(root.join("source")).expect("source");
    std::fs::write(
        root.join("Containerfile"),
        "FROM scratch\nCOPY source /source\n",
    )
    .expect("recipe");
    std::fs::write(root.join("source/tool"), "first").expect("payload");
    fixture.track("Containerfile");
    fixture.track("source");
    fixture.candidate();
    image.build();
    assert_eq!(image.payload(), "first");
    run(Command::new("docker").args(["image", "rm", &image.reference]));
    std::fs::write(root.join("source/tool"), "unstaged").expect("payload");
    std::fs::write(root.join("source/intruder"), "untracked").expect("intruder");
    std::fs::write(root.join(".dockerignore"), "*").expect("ambient ignore");
    image.build();
    assert_eq!(image.payload(), "first");
    run(Command::new("docker").args(["image", "rm", &image.reference]));
    fixture.track("source/tool");
    image.build();
    assert_eq!(image.payload(), "unstaged");
    assert_eq!(
        image.inspect("{{index .Config.Labels \"org.opencontainers.image.revision\"}}"),
        "c".repeat(40)
    );
}

struct Image<'a, 'b> {
    fixture: &'a Fixture<'b>,
    reference: String,
}

#[test]
#[ignore = "explicit local Docker build and native regctl archive/authentication integration"]
fn authentication() {
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
    let name = root
        .file_name()
        .expect("name")
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace('.', "");
    let image = Image {
        fixture: &fixture,
        reference: format!("127.0.0.1:1/plumb-proof/{name}:v1.0.0"),
    };
    std::fs::write(root.join("plumb.toml"), format!(
        "[release]\nproduct = \"probe\"\nauthority = \"https://releases.test\"\n[release.oci]\nregistry = \"127.0.0.1:1\"\nimage = \"plumb-proof/{name}\"\naccount = \"Example\"\n"
    )).expect("profile");
    std::fs::write(
        root.join("Containerfile"),
        "FROM scratch\nLABEL probe=authentication\n",
    )
    .expect("recipe");
    fixture.track("Containerfile");
    image.build();
    super::executable(
        &tools.join("regctl"),
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
            &tools.join(name),
            "#!/bin/sh\ntouch \"$PLUMB_TEST_HELPER\"\nexit 1\n",
        );
    }
    let invoked = root.join("invoked");
    let seats = root.join("seats");
    let output = fixture
        .command()
        .args(["ship", "oci", "publish"])
        .env("PLUMB_RELEASE_VERSION", "v1.0.0")
        .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer fixture")
        .env("PLUMB_TEST_NATIVE", native.trim())
        .env("PLUMB_TEST_SOURCE", root.join("source"))
        .env("PLUMB_TEST_SEATS", &seats)
        .env("PLUMB_TEST_HELPER", &invoked)
        .output()
        .expect("publication");
    assert!(!output.status.success(), "closed registry cannot exist");
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("cannot be read anonymously"), "{error}");
    assert!(!invoked.exists(), "registry read invoked credential helper");
    let seats = std::fs::read_to_string(seats).expect("configurations");
    assert!(
        seats.lines().any(|path| path.ends_with("/anonymous")),
        "{seats}"
    );
    for path in seats.lines() {
        assert!(
            !std::path::Path::new(path).exists(),
            "configuration leaked: {path}"
        );
    }
}

impl Image<'_, '_> {
    fn payload(&self) -> String {
        let created = run(Command::new("docker").args([
            "create",
            "--entrypoint",
            "/source/tool",
            &self.reference,
        ]));
        let container = String::from_utf8(created.stdout)
            .expect("container")
            .trim()
            .to_string();
        let path = self.fixture.root.join("copied");
        let copied = Command::new("docker")
            .args(["cp", &format!("{container}:/source/tool")])
            .arg(&path)
            .output()
            .expect("copy");
        let removed = Command::new("docker")
            .args(["rm", &container])
            .output()
            .expect("remove");
        assert!(
            removed.status.success(),
            "{}",
            String::from_utf8_lossy(&removed.stderr)
        );
        assert!(
            copied.status.success(),
            "{}",
            String::from_utf8_lossy(&copied.stderr)
        );
        std::fs::read_to_string(path).expect("payload")
    }

    fn build(&self) {
        run(self
            .fixture
            .command()
            .args(["ship", "oci", "build"])
            .env("PLUMB_RELEASE_VERSION", "v1.0.0")
            .env("PLUMB_RELEASE_COMMIT", "c".repeat(40)));
    }

    fn inspect(&self, format: &str) -> String {
        let output = run(Command::new("docker").args([
            "image",
            "inspect",
            "--format",
            format,
            &self.reference,
        ]));
        String::from_utf8(output.stdout)
            .expect("identity")
            .trim()
            .to_string()
    }
}

impl Drop for Image<'_, '_> {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["image", "rm", &self.reference])
            .output();
    }
}
