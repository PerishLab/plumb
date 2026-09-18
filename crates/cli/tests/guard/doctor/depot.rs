use std::path::Path;
use std::process::Command;

#[path = "../ownership.rs"]
mod ownership;

fn run(root: &Path, seat: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("path should be utf8")])
        .env_remove("PLUMB_RELEASE_VERSION")
        .env("PLUMB_HOME", seat)
        .output()
        .expect("plumb should run");
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn stock(seat: &Path, mark: &str, manifest: &str) {
    let seat = seat.join("configurations");
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

#[test]
fn absent() {
    let fixture = super::fixture();
    let empty = tempfile::tempdir().expect("seat");
    let out = run(fixture.path(), empty.path());
    assert!(out.contains("cannot read installed rules"), "{out}");
    assert!(out.contains("run plumb configuration install"), "{out}");
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
