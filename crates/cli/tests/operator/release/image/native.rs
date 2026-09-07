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
