use base64::Engine;
use flate2::{Compression, GzBuilder};
use sha2::{Digest, Sha512};
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
    let output = publish(root, "v2.0.0-beta.1", &observed);
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

#[test]
fn held() {
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
    let bytes = std::fs::read(&workload).expect("module workload");
    let digest = base64::engine::general_purpose::STANDARD.encode(Sha512::digest(bytes));
    std::fs::write(
        observed.with_extension("integrity"),
        format!("sha512-{digest}"),
    )
    .expect("registry integrity");
    let output = publish(root, "v1.0.0-beta.1", &observed);
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
        1
    );
    assert!(!calls.contains("publish "));
}

fn archive(path: &std::path::Path) {
    let file = std::fs::File::create(path).expect("workload");
    let encoder = GzBuilder::new()
        .mtime(0)
        .write(file, Compression::default());
    let mut archive = tar::Builder::new(encoder);
    let body = br#"{"name":"held","version":"1.0.0-beta.1","main":"index.js"}"#;
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

fn publish(
    root: &std::path::Path,
    version: &str,
    observed: &std::path::Path,
) -> std::process::Output {
    let home = tempfile::tempdir().unwrap();
    std::fs::write(
        root.join("packages/held/package.json"),
        r#"{"name":"held","version":"1.0.0-beta.1","main":"index.js"}"#,
    )
    .unwrap();
    let binding = crate::marker::prepare(
        root,
        home.path(),
        "[release]\nproduct='probe'\nauthority='https://releases.test'\n[release.npm]\nregistry='https://registry.invalid'\npackages=['held']\n",
        version,
    );
    let git = root.join("bin/git");
    std::fs::write(&git, "#!/bin/sh\nexec /usr/bin/git \"$@\"\n").unwrap();
    std::fs::set_permissions(git, std::fs::Permissions::from_mode(0o755)).unwrap();
    let inventory = crate::support::Bucket::open(3);
    let request = serde_json::json!({
        "schema":"plumb.ship-request/v2", "configuration":binding["configuration"], "profile":binding["profile"],
        "action":"ship/npm.held", "projections":["packages/held/package.json#/version"], "roots":["packages/held"],
        "operation":{"type":"npm","package":"held"},
        "reuse":{"type":"workload","source":"https://inventory.invalid/held.tgz"},
        "keys":{"workload":"1".repeat(64),"proof":"2".repeat(64),"publication":"3".repeat(64)},
    });
    let request = crate::marker::planned(root, home.path(), request, version);
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .current_dir(root)
        .args(["ship", "execute", "--request", &request.to_string()])
        .env("PATH", root.join("bin"))
        .env("PLUMB_HOME", home.path())
        .env("PLUMB_RULES_SOURCE", "https://depot.test")
        .env("PLUMB_TEST_OBSERVED", observed)
        .env("PLUMB_RELEASE_ROOT", root)
        .env("PLUMB_RELEASE_VERSION", version)
        .env("PLUMB_RELEASE_REGISTRY_TOKEN", "Bearer secret")
        .env("PLUMB_WORKFLOW_INVENTORY_ACCESS", "access")
        .env("PLUMB_WORKFLOW_INVENTORY_SECRET", "secret")
        .env("PLUMB_WORKFLOW_INVENTORY_BUCKET", "workflow")
        .env("PLUMB_WORKFLOW_INVENTORY_ENDPOINT", inventory.endpoint())
        .env(
            "PLUMB_WORKFLOW_INVENTORY_URL",
            "https://inventory.invalid/inventory.json",
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    inventory.finish();
    output
}
