use std::fs::File;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

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
    let workload = root.join("probe-1.2.3.tgz");
    archive(&workload);
    let helm = r#"#!/bin/sh
set -eu
if [ "$1" = registry ]; then /bin/cat >/dev/null; exit 0; fi
destination=
while [ $# -gt 0 ]; do
  if [ "$1" = --destination ]; then destination=$2; break; fi
  shift
done
/bin/cp "$PLUMB_TEST_CHART" "$destination/probe-1.2.3.tgz"
"#;
    for (name, body) in [
        ("helm", helm.to_string()),
        ("curl", "#!/bin/sh\n/bin/cat \"$PLUMB_TEST_CHART\"\n".into()),
    ] {
        let path = root.join("bin").join(name);
        std::fs::write(&path, body).expect("fixture tool");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .expect("fixture mode");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "ship",
            "chart",
            "exact",
            "--reuse",
            r#"{"type":"workload","source":"https://inventory.invalid/probe.tgz"}"#,
        ])
        .env("PATH", root.join("bin"))
        .env("PLUMB_TEST_CHART", &workload)
        .env("PLUMB_RELEASE_ROOT", root)
        .env("PLUMB_RELEASE_VERSION", "v1.2.3")
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
    let body = b"apiVersion: v2\nname: probe\nversion: 1.2.3\nappVersion: \"1.2.3\"\n";
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
