use super::{home, seated, track};
use std::path::Path;
use std::process::Command;

const RETIRED: &str = r#"
[layout]

[[layout.seat]]
path = "charts/*"
rule = ["rule://seat/named-after-repository"]

[[layout.seat]]
path = "skills/*"
kind = "retired"

[[layout.seat]]
path = "docs/CHANGELOG/*"
kind = "retired"

[[layout.seat]]
path = ".plumb"

[[layout.file]]
name = ["AGENTS.md"]
rule = ["rule://seat/affirmed"]

[[layout.file]]
name = ["LICENSE", ".gitignore"]

[[layout.file]]
name = ["plumb.toml"]

[[layout.file]]
name = ["ectropy.toml", "runseal.toml", "locus.toml", "sidecar.toml"]
kind = "retired"
"#;

const PRIOR: &str = r#"
[[layout]]
note = "pre-bailiwick standalone tool configs"

[[layout.seat]]
path = "charts/*"
rule = ["rule://seat/named-after-repository"]

[[layout.seat]]
path = "skills/*"
kind = "retired"

[[layout.seat]]
path = "docs/CHANGELOG/*"
kind = "retired"

[[layout.seat]]
path = ".plumb"

[[layout.file]]
name = ["AGENTS.md"]
rule = ["rule://seat/affirmed"]

[[layout.file]]
name = ["LICENSE", ".gitignore"]

[[layout.file]]
name = ["plumb.toml", "ectropy.toml", "runseal.toml", "locus.toml", "sidecar.toml"]
"#;

const MINIMAL: &str = r#"
[layout]

[[layout.seat]]
path = "crates/*"
anchor = ["Cargo.toml"]

[[layout.seat]]
path = "apps/*"
anchor = []

[[layout.seat]]
path = "charts/*"
rule = ["rule://seat/named-after-repository", "rule://seat/wayfinder"]
"#;

fn doctor(root: &Path, depot: &Path) -> (bool, serde_json::Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("utf8"), "--json"])
        .env("PLUMB_HOME", depot)
        .output()
        .expect("plumb");
    let json = serde_json::from_slice(&output.stdout).expect("doctor json");
    (output.status.success(), json)
}

fn write(root: &Path, depot: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["affirm", root.to_str().expect("utf8"), "--write"])
        .env("PLUMB_HOME", depot)
        .output()
        .expect("plumb")
}

fn grades(json: &serde_json::Value) -> Vec<(String, String)> {
    json["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .filter(|item| item["code"].as_str() == Some("structure.seat-affirmed"))
        .map(|item| {
            (
                item["grade"].as_str().unwrap_or_default().to_string(),
                item["evidence"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

#[test]
fn transition() {
    let seat = seated(MINIMAL);
    assert!(write(seat.path(), home()).status.success());
    track(seat.path());
    let before = std::fs::read_to_string(seat.path().join(".plumb/affirmed.toml")).expect("old");

    let grace = crate::support::overlay(&[
        ("rules/suites/layout.toml", RETIRED),
        ("rules/suites/transition.toml", PRIOR),
    ]);
    let (ok, json) = doctor(seat.path(), grace.path());
    assert!(ok, "transitional Depot layout must not fail: {json}");
    let seen = grades(&json);
    assert_eq!(seen.len(), 1, "{json}");
    assert_eq!(seen[0].0, "noted");
    assert!(seen[0].1.contains("transitional Depot layout"), "{seen:?}");
    assert!(
        seen[0].1.contains("pre-bailiwick standalone tool configs"),
        "{seen:?}"
    );
    assert!(seen[0].1.contains("plumb affirm --write"), "{seen:?}");
    assert_eq!(
        std::fs::read_to_string(seat.path().join(".plumb/affirmed.toml")).expect("unchanged"),
        before,
        "doctor must never auto-rewrite .plumb/affirmed.toml"
    );

    let manifest = seat.path().join("plumb.toml");
    std::fs::write(
        &manifest,
        format!("{MINIMAL}\n[[layout.seat]]\npath = \"docs\"\n"),
    )
    .expect("drift");
    track(seat.path());
    let (drifted, json) = doctor(seat.path(), grace.path());
    assert!(!drifted, "repo-local drift must fail closed: {json}");
    let seen = grades(&json);
    assert_eq!(seen.len(), 1, "{json}");
    assert_eq!(seen[0].0, "out of true");
    assert!(
        seen[0].1.contains("was affirmed against a different"),
        "{seen:?}"
    );

    std::fs::write(&manifest, MINIMAL).expect("restore");
    track(seat.path());
    let preview = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["affirm", seat.path().to_str().expect("utf8")])
        .env("PLUMB_HOME", grace.path())
        .output()
        .expect("plumb");
    assert!(preview.status.success());
    assert!(String::from_utf8_lossy(&preview.stdout).contains("authority = "));
    assert_eq!(
        std::fs::read_to_string(seat.path().join(".plumb/affirmed.toml")).expect("still old"),
        before
    );
    assert!(write(seat.path(), grace.path()).status.success());
    track(seat.path());
    let after = std::fs::read_to_string(seat.path().join(".plumb/affirmed.toml")).expect("new");
    assert_ne!(after, before);
    let (settled, json) = doctor(seat.path(), grace.path());
    assert!(settled, "{json}");
    assert!(grades(&json).is_empty(), "{json}");

    let closed = crate::support::overlay(&[
        ("rules/suites/layout.toml", RETIRED),
        ("rules/suites/transition.toml", "layout = []\n"),
    ]);
    let (kept, json) = doctor(seat.path(), closed.path());
    assert!(kept, "{json}");
    assert!(grades(&json).is_empty(), "{json}");

    std::fs::write(seat.path().join(".plumb/affirmed.toml"), &before).expect("stale");
    track(seat.path());
    let (expired, json) = doctor(seat.path(), closed.path());
    assert!(
        !expired,
        "closed transition window must fail closed: {json}"
    );
    let seen = grades(&json);
    assert_eq!(seen.len(), 1, "{json}");
    assert_eq!(seen[0].0, "out of true");
}

#[test]
fn malformed() {
    let seat = seated(MINIMAL);
    let broken = crate::support::overlay(&[("rules/suites/transition.toml", "bogus = 1\n")]);
    let (ok, json) = doctor(seat.path(), broken.path());
    assert!(!ok, "malformed transition must fail closed: {json}");
    let blind = json["findings"]
        .as_array()
        .expect("findings")
        .iter()
        .any(|item| item["grade"].as_str() == Some("blind"));
    assert!(blind, "{json}");

    let failed = write(seat.path(), broken.path());
    assert!(!failed.status.success());
    assert!(!seat.path().join(".plumb/affirmed.toml").exists());
}
