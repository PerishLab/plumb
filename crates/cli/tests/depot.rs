#[path = "support/mod.rs"]
mod support;

use plumb::depot::v3::{self, Bundle, Identity, Kind, Marker, Pointer, Publication, Route};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use support::Bucket;
use tempfile::TempDir;

struct Seed<'a> {
    channel: &'a str,
    version: &'a str,
    leaf: &'a str,
    body: &'a [u8],
}

fn open(requests: usize, version: Option<&str>, skill: bool) -> (Bucket, String, TempDir) {
    let bucket = Bucket::open(requests);
    let authority = format!("{}/workflow", bucket.endpoint());
    let dir = tempfile::tempdir().expect("seat");
    if let Some(version) = version {
        fs::write(
            dir.path().join("Cargo.toml"),
            format!("[workspace.package]\nversion = \"{version}\"\n"),
        )
        .expect("cargo");
    }
    fs::write(
        dir.path().join("plumb.toml"),
        format!(
            "[identity]\nns = \"probe\"\n\n[release]\nauthority = \"https://releases.probe.test\"\nbinaries = [\"probe\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\nskill = {skill}\n\n[release.depot]\nsource = \"{authority}\"\n"
        ),
    )
    .expect("manifest");
    (bucket, authority, dir)
}

fn invoke(root: &Path, extra: &[&str]) -> Output {
    let home = support::home();
    let mut args = vec!["depot", "skill", root.to_str().expect("path")];
    args.extend_from_slice(extra);
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(&args)
        .env("PLUMB_HOME", home.path())
        .output()
        .expect("run")
}

fn lodge(bucket: &Bucket, authority: &str, seed: Seed<'_>) -> (String, String) {
    let source = tempfile::tempdir().expect("source");
    fs::write(source.path().join(seed.leaf), seed.body).expect("leaf");
    let bundle = Bundle::read(
        source.path(),
        Identity {
            product: "probe".into(),
            channel: seed.channel.into(),
            version: seed.version.into(),
            marker: Marker {
                name: seed.version.into(),
                sha256: "a".repeat(64),
            },
            kind: Kind::Skill,
        },
    )
    .expect("bundle");
    let pointer = Pointer::new(
        &bundle.manifest,
        Publication {
            source: authority,
            prior: None,
            created: "2026-09-01T01:02:03Z".into(),
        },
    )
    .expect("pointer");
    let route = Route::new(seed.channel, Kind::Skill, seed.version);
    let prefix = v3::generation(route, &pointer.generation).expect("generation");
    bucket.seed(
        &v3::latest(route).expect("latest"),
        &pointer.encode().expect("pointer"),
    );
    bucket.seed(
        &format!("{prefix}/{}", v3::LEAF),
        &bundle.manifest.encode().expect("manifest"),
    );
    for (path, body) in &bundle.bodies {
        bucket.seed(&format!("{prefix}/objects/{path}"), body);
    }
    (pointer.generation, prefix)
}

fn refused(output: &Output, expected: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(stderr.contains(expected), "{stderr}");
}

#[test]
fn default() {
    let (bucket, authority, root) = open(3, None, true);
    let body = b"# Probe Skill\n\nLodged instructions.\n";
    let (generation, _) = lodge(
        &bucket,
        &authority,
        Seed {
            channel: "stable",
            version: "v1.2.3",
            leaf: "SKILL.md",
            body,
        },
    );
    let output = invoke(root.path(), &["--version", "1.2.3"]);
    bucket.finish();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        format!("v1.2.3 {generation}\n\n# Probe Skill\n\nLodged instructions.\n")
    );
}

#[test]
fn ending() {
    let (bucket, authority, root) = open(3, None, true);
    let (generation, _) = lodge(
        &bucket,
        &authority,
        Seed {
            channel: "stable",
            version: "v1.2.3",
            leaf: "SKILL.md",
            body: b"# No Trailing Newline",
        },
    );
    let output = invoke(root.path(), &["--version", "v1.2.3"]);
    bucket.finish();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        format!("v1.2.3 {generation}\n\n# No Trailing Newline\n")
    );
}

#[test]
fn raw() {
    let (bucket, authority, root) = open(3, Some("1.2.3"), true);
    let body = b"---\nname: probe\n---\nExact bytes without trailing newline";
    lodge(
        &bucket,
        &authority,
        Seed {
            channel: "stable",
            version: "v1.2.3",
            leaf: "SKILL.md",
            body,
        },
    );
    let output = invoke(root.path(), &["--raw"]);
    bucket.finish();
    assert!(output.status.success());
    assert_eq!(output.stdout, body);
}

#[test]
fn prerelease() {
    let (bucket, authority, root) = open(3, None, true);
    let (generation, _) = lodge(
        &bucket,
        &authority,
        Seed {
            channel: "rc",
            version: "v1.2.3-rc.1",
            leaf: "SKILL.md",
            body: b"# RC Skill\n",
        },
    );
    let output = invoke(root.path(), &["--version", "1.2.3-rc.1"]);
    bucket.finish();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        format!("v1.2.3-rc.1 {generation}\n\n# RC Skill\n")
    );
}

#[test]
fn missing() {
    let (bucket, _, root) = open(1, None, true);
    let output = invoke(root.path(), &["--version", "v1.2.3"]);
    bucket.finish();
    refused(&output, "carries no skill for v1.2.3");
}

#[test]
fn corrupted() {
    let (bucket, authority, root) = open(3, None, true);
    let (_, prefix) = lodge(
        &bucket,
        &authority,
        Seed {
            channel: "stable",
            version: "v1.2.3",
            leaf: "SKILL.md",
            body: b"# Authentic\n",
        },
    );
    bucket.seed(&format!("{prefix}/objects/SKILL.md"), b"# Tampered!\n");
    let output = invoke(root.path(), &["--version", "v1.2.3", "--raw"]);
    bucket.finish();
    refused(&output, "depot object drift: SKILL.md");
}

#[test]
fn undeclared() {
    let (bucket, _, root) = open(0, Some("1.2.3"), false);
    let output = invoke(root.path(), &["--version", "v1.2.3"]);
    bucket.finish();
    refused(&output, "declares no skill");
}

#[test]
fn invalid() {
    let (bucket, _, root) = open(0, Some("1.2.3"), true);
    for bad in ["", "not-a-version"] {
        refused(
            &invoke(root.path(), &["--version", bad]),
            "invalid release version",
        );
    }
    bucket.finish();
}

#[test]
fn binary() {
    let (bucket, authority, root) = open(3, None, true);
    lodge(
        &bucket,
        &authority,
        Seed {
            channel: "stable",
            version: "v1.2.3",
            leaf: "SKILL.md",
            body: &[0xff, 0xfe, 0xfd],
        },
    );
    let output = invoke(root.path(), &["--version", "v1.2.3", "--raw"]);
    bucket.finish();
    refused(&output, "not UTF-8");
}

#[test]
fn absent() {
    let (bucket, authority, root) = open(2, None, true);
    lodge(
        &bucket,
        &authority,
        Seed {
            channel: "stable",
            version: "v1.2.3",
            leaf: "OTHER.md",
            body: b"# Not a skill\n",
        },
    );
    let output = invoke(root.path(), &["--version", "v1.2.3"]);
    bucket.finish();
    refused(&output, "carries no SKILL.md");
}
