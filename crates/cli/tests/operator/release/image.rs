use super::world::{Fixture, SPEC};
use flate2::{Compression, GzBuilder, read::GzDecoder};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const DOCKER: &str = "#!/bin/sh\nexit 0\n";
const ARCHIVE: &str = "probe-x86_64-unknown-linux-gnu.tar.gz";

fn seat<'a>(root: &'a std::path::Path, tools: &'a std::path::Path) -> Fixture<'a> {
    Fixture { root, tools }
}

fn build(fixture: &Fixture<'_>, artifacts: &std::path::Path) -> std::process::Output {
    fixture
        .command()
        .args(["ship", "oci", "build"])
        .env("PLUMB_RELEASE_VERSION", "v1.2.0-beta.7")
        .env("PLUMB_RELEASE_COMMIT", "c".repeat(40))
        .env("PLUMB_RELEASE_ARTIFACTS", artifacts)
        .output()
        .expect("plumb should run")
}

#[test]
fn payload() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    let tools = root.join("tools");
    let artifacts = root.join("artifacts");
    let empty = root.join("empty");
    for held in [&tools, &artifacts, &empty] {
        std::fs::create_dir_all(held).expect("fixture root");
    }
    let fixture = seat(root, &tools);
    fixture.seed();
    std::fs::write(
        root.join("plumb.toml"),
        format!(
            "{SPEC}[release.oci]\nregistry = \"example.invalid\"\nimage = \"owner/probe\"\naccount = \"Example\"\n"
        ),
    )
    .expect("manifest");
    std::fs::write(root.join("Containerfile"), "FROM scratch\n").expect("container file");
    let docker = tools.join("docker");
    std::fs::write(&docker, DOCKER).expect("fake docker");
    std::fs::set_permissions(&docker, std::fs::Permissions::from_mode(0o755)).expect("docker mode");
    fixture.archive(&artifacts, "v1.2.0-beta.7");

    let local = build(&fixture, &artifacts);
    let said = String::from_utf8_lossy(&local.stdout).to_string();
    let bytes = std::fs::read(artifacts.join(ARCHIVE)).expect("archive");
    let digest = format!("{:x}", Sha256::digest(&bytes));
    assert!(
        local.status.success() && said.contains(&digest),
        "a local archive is the payload it labels: {said}"
    );

    let object = root.join("releases/v1/objects/sha256").join(&digest);
    std::fs::create_dir_all(object.parent().expect("object root")).expect("object root");
    std::fs::write(&object, &bytes).expect("published object");
    let seal = root.join("releases/v1/releases/beta/v1.2.0-beta.7");
    std::fs::create_dir_all(&seal).expect("seal root");
    let record = |sha: &str| {
        format!(
            concat!(
                r#"{{"artifacts":{{"linux-x64":{{"name":"{}","mime":"application/gzip","#,
                r#""sha256":"{}","size":{},"#,
                r#""url":"https://releases.test/v1/objects/sha256/{}"}}}}}}"#
            ),
            ARCHIVE,
            sha,
            bytes.len(),
            digest
        )
    };
    std::fs::write(seal.join("seal.json"), record(&digest)).expect("published seal");

    let published = build(&fixture, &empty);
    let said = String::from_utf8_lossy(&published.stdout).to_string();
    assert!(
        published.status.success() && said.contains(&digest),
        "an absent archive is taken from the authority that published it: {said}{}",
        String::from_utf8_lossy(&published.stderr)
    );

    std::fs::write(seal.join("seal.json"), record(&"0".repeat(64))).expect("published seal");
    let drifted = build(&fixture, &empty);
    let refused = String::from_utf8_lossy(&drifted.stderr).to_string();
    assert!(
        !drifted.status.success() && refused.contains("published payload drift"),
        "a payload the seal does not name refuses: {refused}"
    );
}

#[test]
fn chart() {
    let temp = tempfile::tempdir().expect("temp root");
    let tools = temp.path().join("tools");
    let fixture = super::lane::seat(temp.path(), &tools);
    std::fs::write(
        temp.path().join("plumb.toml"),
        "[release.chart]\nregistry = \"registry.example\"\nchart = \"owner/probe\"\naccount = \"Example\"\n",
    )
    .expect("release manifest");
    std::fs::create_dir_all(temp.path().join("charts/probe")).expect("chart root");
    std::fs::write(
        temp.path().join("charts/probe/Chart.yaml"),
        "name: probe\nversion: 1.0.0\nappVersion: \"1.0.0\"\n",
    )
    .expect("chart manifest");

    let surface = super::world::run(fixture.command().args(["release", "surface"]));
    let surface: serde_json::Value = serde_json::from_slice(&surface.stdout).expect("surface json");
    let row = surface["project"]["include"]
        .as_array()
        .expect("project rows")
        .iter()
        .find(|row| row["medium"] == "chart")
        .expect("chart row");
    assert_eq!(row["action"], "ship/chart");
    assert_eq!(
        row["projections"],
        serde_json::json!([
            "charts/probe/Chart.yaml#/version",
            "charts/probe/Chart.yaml#/appVersion"
        ])
    );
    let held = super::lane::rendered(&fixture);
    assert!(held.contains("plumb ship chart exact"), "{held}");
    assert!(
        held.contains("matrix.medium != 'npm' && matrix.medium != 'chart'"),
        "the legacy carrier must not also publish the exact chart: {held}"
    );

    let source = temp.path().join("source.tgz");
    packed(&source, "1.0.0");
    executable(
        &tools.join("curl"),
        "#!/bin/sh\nset -eu\ncat \"$FAKE_CHART_WORKLOAD\"\n",
    );
    executable(
        &tools.join("helm"),
        r#"#!/bin/sh
set -eu
case "$1" in
  registry) cat >/dev/null ;;
  pull)
    shift
    destination=
    version=
    while [ $# -gt 0 ]; do
      case "$1" in
        --destination) destination=$2; shift 2 ;;
        --version) version=$2; shift 2 ;;
        *) shift ;;
      esac
    done
    [ -f "$FAKE_CHART_REGISTRY" ] || exit 1
    cp "$FAKE_CHART_REGISTRY" "$destination/probe-$version.tgz"
    ;;
  push) cp "$2" "$FAKE_CHART_REGISTRY" ;;
  show) ;;
  package) exit 97 ;;
  *) exit 98 ;;
esac
"#,
    );
    let registry = temp.path().join("registry.tgz");
    let output = super::world::run(
        fixture
            .command()
            .env("FAKE_CHART_WORKLOAD", &source)
            .env("FAKE_CHART_REGISTRY", &registry)
            .env("PLUMB_RELEASE_VERSION", "v2.0.0")
            .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret")
            .args([
                "ship",
                "chart",
                "exact",
                "--reuse",
                r#"{"type":"workload","source":"https://inventory.example/chart.tgz"}"#,
            ]),
    );
    let projected: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("chart projection");
    assert_eq!(
        projected["publication"],
        "https://registry.example/owner/-/packages/container/probe/2.0.0"
    );
    let workload = projected["workload"].as_str().expect("workload path");
    assert_eq!(manifest(Path::new(workload)), manifest(&registry));
    let chart = manifest(Path::new(workload));
    assert!(chart.contains("version: 2.0.0"), "{chart}");
    assert!(chart.contains("appVersion: \"2.0.0\""), "{chart}");
}

fn packed(path: &Path, version: &str) {
    let file = std::fs::File::create(path).expect("archive");
    let gzip = GzBuilder::new()
        .mtime(0)
        .write(file, Compression::default());
    let mut archive = tar::Builder::new(gzip);
    let body = format!("name: probe\nversion: {version}\nappVersion: \"{version}\"\n");
    let mut header = tar::Header::new_gnu();
    header.set_mode(0o644);
    header.set_size(body.len() as u64);
    header.set_cksum();
    archive
        .append_data(&mut header, "probe/Chart.yaml", body.as_bytes())
        .expect("chart entry");
    archive.into_inner().expect("tar").finish().expect("gzip");
}

fn manifest(path: &Path) -> String {
    let file = std::fs::File::open(path).expect("archive");
    let mut archive = tar::Archive::new(GzDecoder::new(file));
    for entry in archive.entries().expect("entries") {
        let mut entry = entry.expect("entry");
        if entry.path().expect("path") == Path::new("probe/Chart.yaml") {
            let mut text = String::new();
            entry.read_to_string(&mut text).expect("manifest");
            return text;
        }
    }
    panic!("chart archive carries no manifest")
}

fn executable(path: &Path, body: &str) {
    std::fs::write(path, body).expect("fake tool");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("tool mode");
}
