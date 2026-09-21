use std::process::Command;

const BINARY: &str = "[release]\nproduct = \"foo\"\nauthority = \"https://example.invalid\"\nbinaries = [\"foo\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n";

#[test]
fn platforms() {
    let fixture = tempfile::tempdir().expect("fixture");
    crate::govern(fixture.path());
    let path = fixture.path().to_str().expect("path");
    for triple in [
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
        "x86_64-pc-windows-msvc",
        "x86_64-apple-darwin",
    ] {
        std::fs::write(
            fixture.path().join("plumb.toml"),
            BINARY.replace("x86_64-unknown-linux-gnu", triple),
        )
        .expect("manifest");
        let report = crate::run(&["doctor", path]);
        assert_eq!(
            report.contains("unsupported Ship target"),
            triple == "x86_64-apple-darwin",
            "{triple}: {report}"
        );
    }
}

fn seat(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    crate::govern(&dir);
    dir
}

#[test]
fn refused() {
    let dir = seat("plumb-release-declaration");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(dir.join("plumb.toml"), BINARY).expect("manifest should be written");
    let whole = crate::run(&["doctor", path]);
    assert!(!whole.contains("the current Plumb refuses"), "{whole}");
    assert!(whole.contains("publishes binary"), "{whole}");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release]\nproduct = \"foo\"\nauthority = \"https://example.invalid\"\nbinaries = [\"foo\"]\n[release.npm]\nregistry = \"https://example.invalid/npm/\"\npackages = [\"@probe/foo\"]\n\ntargets = [\"x86_64-unknown-linux-gnu\"]\n",
    )
    .expect("manifest should be written");
    let captured = crate::run(&["doctor", "--json", path]);
    let report: serde_json::Value = serde_json::from_str(&captured).expect("doctor json");
    assert_eq!(report["summary"]["out_of_true"], 1, "{captured}");
    assert!(
        report["findings"]
            .as_array()
            .is_some_and(|findings| findings.iter().any(|finding| {
                finding["code"] == "release.spec-declared"
                    && finding["evidence"].as_str().is_some_and(|evidence| {
                        evidence.contains("the current Plumb refuses")
                            && evidence.contains("cannot parse")
                    })
            })),
        "{captured}"
    );

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn deliverable() {
    let dir = seat("plumb-release-attachment");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(dir.join("plumb.toml"), BINARY).expect("manifest should be written");
    let uncalled = crate::run(&["doctor", path]);
    assert!(uncalled.contains("publishes binary"), "{uncalled}");
    assert!(!uncalled.contains("attachment is declared"), "{uncalled}");
    assert!(!dir.join(".forgejo").exists());

    std::fs::write(
        dir.join("plumb.toml"),
        format!(
            "{BINARY}\n[release.oci]\nregistry = \"example.invalid\"\nimage = \"perishlab/foo\"\naccount = \"Example\"\n"
        ),
    )
    .expect("manifest should be written");
    let held = crate::ruled(&[("rules/release.toml", RELEASE)], &["doctor", path]);
    assert!(held.contains("publishes binary oci"), "{held}");
    assert!(!held.contains("attachment is declared"), "{held}");

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn carried() {
    let dir = seat("plumb-release-cargo-only");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release.cargo]\nregistry = \"perish\"\npackages = [\"foo\"]\n",
    )
    .expect("manifest should be written");
    let held = crate::ruled(&[("rules/release.toml", RELEASE)], &["doctor", path]);
    assert!(held.contains("publishes cargo"), "{held}");
    assert!(!held.contains("the current Plumb refuses"), "{held}");
    assert!(!held.contains("cargo attachment is declared"), "{held}");

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn depot() {
    let dir = seat("plumb-release-depot");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(
        dir.join("plumb.toml"),
        format!(
            "{BINARY}\n[release.depot]\nsource = \"https://depot.foo.example\"\nderivatives = [\"configuration\", \"changelog\"]\nvalidator = [\"foo\", \"guard\"]\n"
        ),
    )
    .expect("manifest should be written");
    let held = crate::ruled(&[("rules/release.toml", RELEASE)], &["doctor", path]);
    assert!(!held.contains("the current Plumb refuses"), "{held}");

    std::fs::write(
        dir.join("plumb.toml"),
        format!(
            "{BINARY}\n[release.depot]\nsource = \"https://depot.foo.example\"\nderivatives = [\"artifact\"]\n"
        ),
    )
    .expect("manifest should be written");
    let unknown = crate::run(&["doctor", path]);
    assert!(unknown.contains("the current Plumb refuses"), "{unknown}");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release]\nproduct = \"foo\"\nauthority = \"https://releases.foo.example\"\n[release.cargo]\nregistry = \"perish\"\npackages = [\"foo\"]\n[release.depot]\nsource = \"https://depot.foo.example\"\nderivatives = [\"configuration\"]\nvalidator = [\"foo\", \"guard\"]\n",
    )
    .expect("manifest should be written");
    let source = crate::run(&["doctor", path]);
    assert!(
        source.contains("configuration derivative requires an exact released binary"),
        "{source}"
    );

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn attached() {
    let dir = seat("plumb-release-attachment-only");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release.oci]\nregistry = \"git.perish.top\"\nimage = \"owner/name\"\naccount = \"PerishFire\"\n",
    )
    .expect("manifest should be written");
    let image = crate::run(&["doctor", path]);
    assert!(image.contains("publishes oci"), "{image}");
    assert!(!image.contains("the current Plumb refuses"), "{image}");

    std::fs::write(dir.join("plumb.toml"), "[release]\nskill = false\n")
        .expect("manifest should be written");
    let empty = crate::run(&["doctor", path]);
    assert!(empty.contains("the current Plumb refuses"), "{empty}");
    assert!(empty.contains("at least one attachment"), "{empty}");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release]\nskill = true\n[release.cargo]\nregistry = \"perish\"\npackages = [\"foo\"]\n",
    )
    .expect("manifest should be written");
    let skill = crate::run(&["doctor", path]);
    assert!(skill.contains("requires a binary release"), "{skill}");

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

const RELEASE: &str =
    "ceiling = 10\n\n[forge]\nimage = \"fixture\"\n\n[permitted]\n\n[exercised]\nnpm = 1\n";

#[test]
fn width() {
    let dir = seat("plumb-release-width");
    let path = dir.to_str().expect("path should be utf8");
    let names: Vec<String> = (1..=11).map(|index| format!("p{index}")).collect();
    for package in &names {
        std::fs::create_dir_all(dir.join("packages").join(package)).expect("module seat");
    }
    let declare = |packages: &str| {
        std::fs::write(
            dir.join("plumb.toml"),
            format!(
                "{BINARY}\n[release.npm]\nregistry = \"https://example.invalid/npm/\"\npackages = {packages}\n"
            ),
        )
        .expect("manifest should be written");
    };

    let list = |count: usize| {
        let held: Vec<String> = names[..count]
            .iter()
            .map(|name| format!("\"@probe/{name}\""))
            .collect();
        format!("[{}]", held.join(", "))
    };

    declare(&list(1));
    let held = crate::ruled(&[("rules/release.toml", RELEASE)], &["doctor", path]);
    assert!(!held.contains("attachment declares"), "{held}");

    declare(&list(10));
    let edge = crate::ruled(&[("rules/release.toml", RELEASE)], &["doctor", path]);
    assert!(!edge.contains("and Plumb permits"), "{edge}");

    declare(&list(11));
    let wide = crate::ruled(&[("rules/release.toml", RELEASE)], &["doctor", path]);
    assert!(
        wide.contains("the npm attachment declares 11 packages and Plumb permits 10"),
        "{wide}"
    );
}

#[test]
fn unsettled() {
    let fixture = tempfile::tempdir().expect("fixture");
    let root = fixture.path();
    crate::govern(root);
    std::fs::write(root.join("plumb.toml"), BINARY).expect("manifest");
    let git = |args: &[&str]| {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args([
                "-c",
                "user.name=Plumb",
                "-c",
                "user.email=plumb@example.invalid",
            ])
            .args(args)
            .status()
            .expect("git should run");
        assert!(status.success(), "git {args:?}");
    };
    git(&["symbolic-ref", "HEAD", "refs/heads/main"]);
    git(&["commit", "-q", "--no-verify", "--allow-empty", "-m", "base"]);
    git(&["checkout", "-q", "-b", "release/v1.0.0"]);
    git(&["commit", "-q", "--no-verify", "--allow-empty", "-m", "line"]);
    git(&["tag", "-a", "v1.0.0", "-m", "v1.0.0"]);
    git(&["checkout", "-q", "-b", "release/v2.0.0", "main"]);
    git(&[
        "commit",
        "-q",
        "--no-verify",
        "--allow-empty",
        "-m",
        "candidate",
    ]);
    git(&["tag", "-a", "v2.0.0-rc.1", "-m", "v2.0.0-rc.1"]);
    git(&["checkout", "-q", "main"]);
    let path = root.to_str().expect("path");

    let held = crate::run(&["doctor", path]);
    assert!(held.contains("noted: stable v1.0.0 at"), "{held}");
    assert!(held.contains("0 out of true"), "{held}");
    assert!(!held.contains("v2.0.0"), "{held}");

    git(&[
        "merge",
        "-q",
        "--no-verify",
        "--no-ff",
        "release/v1.0.0",
        "-m",
        "settle v1.0.0",
    ]);
    let held = crate::run(&["doctor", path]);
    assert!(!held.contains("is not an ancestor of main"), "{held}");
}
