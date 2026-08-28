use flate2::{Compression, GzBuilder};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

const NPM: &str = r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> "$PLUMB_TEST_OBSERVED"
if [ "$1" = view ]; then
  test -f "$PLUMB_TEST_OBSERVED.integrity" || exit 1
  /bin/cat "$PLUMB_TEST_OBSERVED.integrity"
  exit 0
fi
test "$1" = publish
printf 'sha512-' > "$PLUMB_TEST_OBSERVED.integrity"
/usr/bin/openssl dgst -sha512 -binary "$2" | /usr/bin/openssl base64 -A >> "$PLUMB_TEST_OBSERVED.integrity"
"#;

#[test]
fn direct() {
    let fixture = tempfile::tempdir().expect("module fixture");
    let root = fixture.path();
    std::fs::create_dir_all(root.join("packages/held")).expect("module seat");
    std::fs::create_dir_all(root.join("bin")).expect("binary seat");
    std::fs::write(
        root.join("plumb.toml"),
        "[release.npm]\nregistry = \"https://registry.invalid\"\npackages = [\"held\"]\n",
    )
    .expect("release shape");
    let workload = root.join("workload.tgz");
    archive(&workload);
    let curl = format!("#!/bin/sh\n/bin/cat '{}'\n", workload.display());
    for (name, body) in [("npm", NPM), ("curl", curl.as_str())] {
        let path = root.join("bin").join(name);
        std::fs::write(&path, body).expect("fixture binary");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .expect("fixture mode");
    }
    let observed = root.join("observed");
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args([
            "ship",
            "npm",
            "exact",
            "--package",
            "held",
            "--reuse",
            r#"{"type":"workload","source":"https://inventory.invalid/held.tgz"}"#,
        ])
        .env("PATH", root.join("bin"))
        .env("PLUMB_TEST_OBSERVED", &observed)
        .env("PLUMB_RELEASE_ROOT", root)
        .env("PLUMB_RELEASE_VERSION", "v2.0.0-beta.1")
        .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret")
        .output()
        .expect("plumb should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = std::fs::read_to_string(observed).expect("npm calls");
    assert_eq!(
        calls
            .lines()
            .filter(|line| line.starts_with("view "))
            .count(),
        2
    );
    assert_eq!(
        calls
            .lines()
            .filter(|line| line.starts_with("publish "))
            .count(),
        1
    );
}

fn archive(path: &std::path::Path) {
    let file = std::fs::File::create(path).expect("workload");
    let encoder = GzBuilder::new()
        .mtime(0)
        .write(file, Compression::default());
    let mut archive = tar::Builder::new(encoder);
    let body = br#"{"name":"held","version":"1.0.0","main":"index.js"}"#;
    let mut header = tar::Header::new_gnu();
    header
        .set_path("package/package.json")
        .expect("manifest path");
    header.set_size(body.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    archive.append(&header, body.as_slice()).expect("manifest");
    archive
        .into_inner()
        .and_then(flate2::write::GzEncoder::finish)
        .expect("archive");
}
