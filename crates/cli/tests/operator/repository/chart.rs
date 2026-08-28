use sha2::{Digest, Sha256};
use std::fs::{File, FileTimes};
use std::process::Command;
use std::time::{Duration, SystemTime};

#[path = "../../support.rs"]
mod support;

fn package(root: &std::path::Path, depot: &std::path::Path) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["ship", "chart", "package"])
        .env("PLUMB_DEPOT_SEAT", depot)
        .env("PLUMB_RELEASE_ROOT", root)
        .env("PLUMB_RELEASE_VERSION", "v1.2.3")
        .output()
        .expect("plumb should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let archive = root.join("target/chart/probe-1.2.3.tgz");
    let body = std::fs::read(archive).expect("chart archive");
    Sha256::digest(body).to_vec()
}

#[test]
fn stable() {
    let fixture = tempfile::tempdir().expect("fixture");
    let depot = support::depot(&[]);
    let root = fixture.path();
    std::fs::create_dir_all(root.join("charts/probe/templates")).expect("chart seat");
    std::fs::write(
        root.join("plumb.toml"),
        "[release.chart]\nregistry = \"example.invalid\"\nchart = \"owner/probe\"\naccount = \"Example\"\n",
    )
    .expect("release manifest");
    std::fs::write(
        root.join("charts/probe/Chart.yaml"),
        "apiVersion: v2\nname: probe\nversion: 0.0.0\nappVersion: \"0.0.0\"\n",
    )
    .expect("chart manifest");
    std::fs::write(
        root.join("charts/probe/templates/probe.yaml"),
        "kind: ConfigMap\nmetadata:\n  name: probe\n",
    )
    .expect("chart body");

    let first = package(root, depot.path());
    let file = File::open(root.join("charts/probe/Chart.yaml")).expect("chart manifest");
    file.set_times(
        FileTimes::new().set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000)),
    )
    .expect("chart mtime");
    let second = package(root, depot.path());
    assert_eq!(first, second);
}
