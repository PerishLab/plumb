use super::command::plumb;
use std::path::Path;

const DECLARATION: &str = r#"[release]
product = "demo"
authority = "https://releases.demo.test"
binaries = ["demo"]
targets = ["x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
"#;

fn rendered(root: &Path, version: &str) -> bool {
    std::fs::write(root.join("plumb.toml"), DECLARATION).expect("declaration");
    plumb(
        root,
        &[
            "release",
            "managers",
            "--version",
            version,
            "--out",
            root.join("out").to_str().expect("path"),
        ],
    )
    .output()
    .expect("plumb")
    .status
    .success()
}

fn read(root: &Path, leaf: &str) -> String {
    std::fs::read_to_string(root.join("out").join(leaf)).expect(leaf)
}

#[test]
fn canonical() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    assert!(rendered(root, "v1.2.0"));
    let pinned = read(root, "manage.sh");
    let canonical = read(root, "canonical/manage.sh");
    assert!(
        pinned.contains("VERSION=${DEMO_VERSION:-v1.2.0}"),
        "{pinned}"
    );
    assert!(
        canonical.contains("VERSION=${DEMO_VERSION:-}")
            && canonical.contains("CHANNEL=${DEMO_CHANNEL:-stable}"),
        "{canonical}"
    );
    assert!(root.join("out/manage.ps1").is_file());
    assert!(root.join("out/canonical/manage.ps1").is_file());
}

#[test]
fn pinned() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    assert!(rendered(root, "v1.2.0-rc.1"));
    let pinned = read(root, "manage.sh");
    assert!(
        pinned.contains("CHANNEL=${DEMO_CHANNEL:-rc}")
            && pinned.contains("VERSION=${DEMO_VERSION:-v1.2.0-rc.1}"),
        "{pinned}"
    );
    assert!(!root.join("out/canonical").exists());
    assert!(
        !rendered(root, "v1.2.0-rc.1"),
        "an existing output seat is refused"
    );
}
