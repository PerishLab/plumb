use sha2::{Digest, Sha256};
use std::fs::{File, FileTimes};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::time::{Duration, SystemTime};

use super::support;

fn package(root: &std::path::Path, home: &std::path::Path) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["ship", "chart", "package"])
        .env("PLUMB_HOME", home)
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

#[test]
fn reused() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    std::fs::create_dir_all(root.join("bin")).expect("binary seat");
    std::fs::create_dir_all(root.join("charts/probe")).expect("chart seat");
    std::fs::write(
        root.join("plumb.toml"),
        "[release.chart]\nregistry = \"example.invalid\"\nchart = \"owner/probe\"\naccount = \"Example\"\n",
    )
    .expect("release manifest");
    let home = tempfile::tempdir().unwrap();
    std::fs::write(
        root.join("charts/probe/Chart.yaml"),
        "name: probe\nversion: 1.2.3\nappVersion: \"1.2.3\"\n",
    )
    .unwrap();
    let binding = crate::marker::prepare(
        root,
        home.path(),
        "[release]\nproduct='probe'\nauthority='https://releases.test'\n[release.chart]\nregistry='example.invalid'\nchart='owner/probe'\naccount='Example'\n",
        "v1.2.3-beta.1",
    );
    let workload = root.join("probe-1.2.3-beta.1.tgz");
    archive(&workload);
    let digest = plumb::depot::sha(&std::fs::read(&workload).unwrap());
    let helm = r#"#!/bin/sh
set -eu
if [ "$1" = registry ]; then /bin/cat >/dev/null; exit 0; fi
destination=
while [ $# -gt 0 ]; do
  if [ "$1" = --destination ]; then destination=$2; break; fi
  shift
done
/bin/cp "$PLUMB_TEST_CHART" "$destination/probe-1.2.3-beta.1.tgz"
"#;
    for (name, body) in [
        ("git", "#!/bin/sh\nexec /usr/bin/git \"$@\"\n".into()),
        ("helm", helm.to_string()),
        ("curl", "#!/bin/sh\n/bin/cat \"$PLUMB_TEST_CHART\"\n".into()),
    ] {
        let path = root.join("bin").join(name);
        std::fs::write(&path, body).expect("fixture tool");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .expect("fixture mode");
    }
    let request = serde_json::json!({
        "schema":"plumb.ship-request/v3", "marker":"v1.2.3-beta.1", "configuration":binding["configuration"], "profile":binding["profile"],
        "action":"ship/chart", "operation":{"type":"chart"},
        "reuse":{"type":"workload","source":format!("https://blob.test/v2/blobs/sha256/{digest}")},
    });
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .current_dir(root)
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_RULES_SOURCE", "https://depot.test")
        .args(["ship", "execute", "--request", &request.to_string()])
        .env("PATH", root.join("bin"))
        .env("PLUMB_TEST_CHART", &workload)
        .env("PLUMB_RELEASE_ROOT", root)
        .env("PLUMB_RELEASE_VERSION", "v1.2.3-beta.1")
        .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret")
        .output()
        .expect("plumb should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn archive(path: &std::path::Path) {
    let file = File::create(path).expect("chart workload");
    let gzip = flate2::GzBuilder::new()
        .mtime(0)
        .write(file, flate2::Compression::default());
    let mut archive = tar::Builder::new(gzip);
    let body =
        b"apiVersion: v2\nname: probe\nversion: 1.2.3-beta.1\nappVersion: \"1.2.3-beta.1\"\n";
    let mut header = tar::Header::new_gnu();
    header.set_size(body.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    archive
        .append_data(&mut header, "probe/Chart.yaml", body.as_slice())
        .expect("chart manifest");
    archive
        .into_inner()
        .and_then(flate2::write::GzEncoder::finish)
        .expect("chart workload");
}
