use std::process::Command;

const DECLARED: &str = r#"
[[document]]
strategy = "agent"
source = [{ path = ".", seal = "0" }]
target-seal = "0"

[layout]

[[layout.seat]]
path = "crates/*"
anchor = ["Cargo.toml"]
note = "members earn the seat by carrying a manifest"

[[layout.seat]]
path = "apps/*"
anchor = []
note = "governed, members unjudged"

[[layout.seat]]
path = "charts/*"
rule = ["rule://seat/named-after-repository", "rule://seat/wayfinder"]
note = "one member, named after the repository, carrying a capped brief"

[[layout.file]]
name = ["plumb.toml", "AGENTS.md"]
note = "the governance pair"
"#;

fn seated(held: &str) -> tempfile::TempDir {
    let seat = super::fixture();
    std::fs::write(seat.path().join("plumb.toml"), held).expect("plumb.toml");
    std::fs::write(seat.path().join("AGENTS.md"), "# Agents\n").expect("AGENTS.md");
    for name in ["crates/cli", "apps/web"] {
        std::fs::create_dir_all(seat.path().join(name)).expect("member");
        std::fs::write(seat.path().join(name).join("kept"), "held").expect("leaf");
    }
    std::fs::write(seat.path().join("crates/cli/Cargo.toml"), "").expect("anchor");
    let named = seat
        .path()
        .file_name()
        .map(|held| held.to_string_lossy().to_string())
        .expect("fixture name");
    std::fs::create_dir_all(seat.path().join("charts").join(&named)).expect("chart");
    std::fs::write(
        seat.path().join("charts").join(&named).join("SKILL.md"),
        "# held\n",
    )
    .expect("leaf");
    track(seat.path());
    seat
}

fn track(root: &std::path::Path) {
    let status = Command::new("git")
        .args(["-C", root.to_str().expect("utf8"), "add", "."])
        .status()
        .expect("git");
    assert!(status.success(), "fixture should be tracked");
}

fn report(root: &std::path::Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", root.to_str().expect("utf8")])
        .output()
        .expect("plumb");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn covered() {
    let seat = seated(DECLARED);
    let held = report(seat.path());
    assert!(!held.contains("sits in no declared seat"), "{held}");
    assert!(!held.contains("carries none of"), "{held}");
}

#[test]
fn tombstone() {
    let declared = format!(
        "{DECLARED}\n[[layout.seat]]\npath = \"docs/CHANGELOG/*\"\nkind = \"retired\"\nnote = \"history moved to the depot\"\n"
    );
    let seat = seated(&declared);
    let held = report(seat.path());
    assert!(!held.contains("retired seat"), "{held}");
}
