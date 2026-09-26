use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use plumb::depot::v3::{Bundle, Identity, Kind, Marker, Pointer, Publication, Route};

fn seat(name: &str, version: Option<&str>) -> PathBuf {
    let path = std::env::temp_dir().join(format!("plumb-changelog-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("seat");
    if let Some(version) = version {
        fs::write(
            path.join("Cargo.toml"),
            format!("[workspace.package]\nversion = \"{version}\"\n"),
        )
        .expect("cargo");
    }
    path
}

fn marked(workspace: &Path, extra: &[&str]) -> (String, bool) {
    let mut args = vec!["changelog", workspace.to_str().expect("path")];
    args.extend_from_slice(extra);
    let out = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(&args)
        .env("PLUMB_HOME", crate::support::home().keep())
        .output()
        .expect("run");
    let shown = String::from_utf8_lossy(&out.stdout).to_string();
    (shown, out.status.success())
}

#[test]
fn undeclared() {
    let root = seat("undeclared", None);
    let (shown, held) = marked(&root, &[]);
    let _ = fs::remove_dir_all(&root);
    assert!(!held, "{shown}");
    assert!(shown.contains("no version to read"), "{shown}");
}

#[test]
fn invalid() {
    let root = seat("invalid", Some("1.2.3"));
    let (shown, held) = marked(&root, &["--version", "not-a-version"]);
    let _ = fs::remove_dir_all(&root);
    assert!(!held, "{shown}");
    assert!(shown.contains("invalid version"), "{shown}");
}

#[test]
fn generation() {
    let bucket = super::support::Bucket::open(3);
    let authority = format!("{}/workflow", bucket.endpoint());
    let root = seat("generation", None);
    fs::write(
        root.join("plumb.toml"),
        format!(
            r#"[release]
product = "probe"
authority = "https://releases.probe.test"
binaries = ["probe"]
targets = ["x86_64-unknown-linux-gnu"]

[release.depot]
source = "{authority}"
"#
        ),
    )
    .expect("governance");
    let source = tempfile::tempdir().expect("source");
    fs::write(
        source.path().join("CHANGELOG.md"),
        "# v1.2.3\n\nGeneration-backed notes.\n",
    )
    .expect("changelog");
    let version = "v1.2.3";
    let bundle = Bundle::read(
        source.path(),
        Identity {
            product: "probe".into(),
            channel: "stable".into(),
            version: version.into(),
            marker: Marker {
                name: version.into(),
                sha256: "a".repeat(64),
            },
            kind: Kind::Changelog,
        },
    )
    .expect("bundle");
    let pointer = Pointer::new(
        &bundle.manifest,
        Publication {
            source: &authority,
            prior: None,
            created: "2026-09-01T01:02:03Z".into(),
        },
    )
    .expect("pointer");
    let route = Route::new("stable", Kind::Changelog, version);
    let generation = plumb::depot::v3::generation(route, &pointer.generation).expect("generation");
    bucket.seed(
        &plumb::depot::v3::latest(route).expect("latest"),
        &pointer.encode().expect("pointer"),
    );
    bucket.seed(
        &format!("{generation}/{}", plumb::depot::v3::LEAF),
        &bundle.manifest.encode().expect("manifest"),
    );
    for (path, body) in &bundle.bodies {
        bucket.seed(&format!("{generation}/objects/{path}"), body);
    }

    let (shown, held) = marked(&root, &["--version", version]);
    let _ = fs::remove_dir_all(&root);
    bucket.finish();
    assert!(held, "{shown}");
    assert!(shown.contains(&pointer.generation), "{shown}");
    assert!(shown.contains("CHANGELOG.md"), "{shown}");
    assert!(shown.contains("Generation-backed notes."), "{shown}");
}

fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(root)
        .args(["-c", "user.name=probe", "-c", "user.email=probe@test"])
        .args(args)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn note(lines: usize) -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("note");
    for tongue in ["en", "zh"] {
        fs::create_dir_all(home.path().join(tongue)).expect("tongue");
        fs::write(
            home.path().join(tongue).join("INDEX.md"),
            "line\n".repeat(lines),
        )
        .expect("index");
        fs::write(
            home.path().join(tongue).join("MIGRATION.md"),
            "# Migration\n",
        )
        .expect("migration");
    }
    home
}

#[test]
fn prove() {
    let root = seat("prove", None);
    git(&root, &["init", "-q"]);
    fs::write(root.join("a.txt"), "one\n").expect("first");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "first"]);
    git(&root, &["tag", "v1.0.0"]);
    fs::write(root.join("a.txt"), "one\ntwo\n").expect("second");
    git(&root, &["commit", "-qam", "second"]);
    git(&root, &["tag", "-a", "v1.1.0", "-m", "v1.1.0"]);
    let prove = |home: &Path| {
        Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args([
                "changelog",
                root.to_str().expect("path"),
                "--version",
                "1.1.0",
                "--prove",
            ])
            .arg(home)
            .env("PLUMB_HOME", crate::support::home().keep())
            .output()
            .expect("run")
    };
    let within = prove(note(10).path());
    let above = prove(note(200).path());
    let _ = fs::remove_dir_all(&root);
    let shown = String::from_utf8_lossy(&within.stdout);
    assert!(
        within.status.success(),
        "{shown}{}",
        String::from_utf8_lossy(&within.stderr)
    );
    assert!(
        shown.contains("en: 11 lines within a budget of 120 for 2 units"),
        "{shown}"
    );
    assert!(!above.status.success());
    assert!(String::from_utf8_lossy(&above.stderr).contains("above diff budget 120"));
}
