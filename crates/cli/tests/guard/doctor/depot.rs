use std::path::Path;
use std::process::Command;

fn run(root: &Path, seat: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .env_remove("PLUMB_RELEASE_VERSION")
        .env("PLUMB_DEPOT_SEAT", seat)
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn record(root: &Path, path: &str, text: &str) {
    let file = root.join(path);
    std::fs::create_dir_all(file.parent().expect("parent")).expect("seat should be made");
    std::fs::write(&file, text).expect("file should be written");
    let status = Command::new("git")
        .args(["add", path])
        .current_dir(root)
        .status()
        .expect("git should run");
    assert!(status.success(), "fixture should record {path}");
}

fn stock(seat: &Path, mark: &str, manifest: &str) {
    std::fs::create_dir_all(seat.join(mark)).expect("seat should be made");
    std::fs::write(seat.join(mark).join("plumb.toml"), manifest).expect("manifest");
    std::fs::write(
        seat.join("metadata.json"),
        format!(
            "{{\"format\":1,\"product\":\"plumb\",\"channel\":\"stable\",\"version\":\"{mark}\",\"source\":\"https://depot.plumb.perish.uk\",\"commit\":\"\"}}"
        ),
    )
    .expect("pointer");
}

fn roots(root: &Path) {
    record(root, "crates/cli/rules/structure.toml", "dirs = []\n");
    record(root, "crates/lib/rules/vocabulary.toml", "schema = 1\n");
    record(root, "crates/cli/assets/guard/lane.yml.in", "name: guard\n");
}

#[test]
fn factory() {
    let fixture = super::fixture();
    let empty = tempfile::tempdir().expect("seat");
    roots(fixture.path());
    let out = run(fixture.path(), empty.path());
    assert!(!out.contains("[depot]"), "{out}");
}

#[test]
fn floor() {
    let fixture = super::fixture();
    let seat = tempfile::tempdir().expect("seat");
    stock(
        seat.path(),
        "29990101T000000Z",
        "[schema]\nformat = 1\nversion = \"v99.0.0\"\n\n[metadata]\nversion = \"29990101T000000Z\"\nsource = \"https://depot.plumb.perish.uk\"\nchannel = \"stable\"\ncommit = \"\"\n",
    );
    let out = run(fixture.path(), seat.path());
    assert!(out.contains("declares a floor of v99.0.0"), "{out}");
}

#[test]
fn drift() {
    let fixture = super::fixture();
    let seat = tempfile::tempdir().expect("seat");
    roots(fixture.path());
    stock(
        seat.path(),
        "29990101T000000Z",
        "[schema]\nformat = 1\nversion = \"v0.0.1\"\n\n[metadata]\nversion = \"29990101T000000Z\"\nsource = \"https://depot.plumb.perish.uk\"\nchannel = \"stable\"\ncommit = \"\"\n\n[[object]]\npath = \"rules/structure.toml\"\nsha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\nsize = 1\n",
    );
    let out = run(fixture.path(), seat.path());
    assert!(
        out.contains("carries a different rules/structure.toml"),
        "{out}"
    );
    assert!(out.contains("carries no rules/vocabulary.toml"), "{out}");
    assert!(out.contains("0 out of true"), "{out}");
    assert!(out.contains("noted"), "{out}");
}
