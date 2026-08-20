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
fn stray() {
    let seat = seated(DECLARED);
    std::fs::create_dir_all(seat.path().join("novel")).expect("novel");
    std::fs::write(seat.path().join("novel/kept"), "held").expect("leaf");
    std::fs::write(seat.path().join("stray.json"), "{}").expect("stray");
    track(seat.path());
    let held = report(seat.path());
    assert!(
        held.contains("directory novel sits in no declared seat"),
        "{held}"
    );
    assert!(
        held.contains("file stray.json sits in no declared seat"),
        "{held}"
    );
}

#[test]
fn naked() {
    let seat = seated(DECLARED);
    std::fs::create_dir_all(seat.path().join("crates/bare")).expect("bare");
    std::fs::write(seat.path().join("crates/bare/kept"), "held").expect("leaf");
    track(seat.path());
    let held = report(seat.path());
    assert!(
        held.contains("crates/bare carries none of Cargo.toml"),
        "{held}"
    );
    assert!(!held.contains("apps/web carries none of"), "{held}");
}

#[test]
fn refused() {
    let seat = seated("[layout]\n\n[[layout.seat]]\npath = \"crates/*\"\nnovel = 1\n");
    let held = report(seat.path());
    assert!(
        held.contains("layout carries no field called novel"),
        "{held}"
    );
}

#[test]
fn ruled() {
    let seat = seated(DECLARED);
    std::fs::create_dir_all(seat.path().join("charts/novel")).expect("novel");
    std::fs::write(seat.path().join("charts/novel/kept"), "held").expect("leaf");
    track(seat.path());
    let held = report(seat.path());
    assert!(
        held.contains("charts holds 2 members where rule://seat/named-after-repository fixes 1"),
        "{held}"
    );
    assert!(
        held.contains("charts/novel is not named as rule://seat/named-after-repository requires"),
        "{held}"
    );
}

#[test]
fn unread() {
    for held in ["rule://seat/nosuch", "rule://nosuchset", "file://x"] {
        let seat = seated(&DECLARED.replace("rule://seat/named-after-repository", held));
        let shown = report(seat.path());
        assert!(shown.contains("blind"), "{shown}");
        assert!(shown.contains(held), "{shown}");
    }
}

#[test]
fn capped() {
    let seat = seated(DECLARED);
    let named = seat
        .path()
        .file_name()
        .map(|held| held.to_string_lossy().to_string())
        .expect("fixture name");
    let leaf = seat.path().join("charts").join(&named).join("SKILL.md");
    std::fs::write(&leaf, "x".repeat(4096)).expect("fat leaf");
    track(seat.path());
    let held = report(seat.path());
    assert!(
        held.contains("carries 4096 bytes where rule://seat/wayfinder caps 3072"),
        "{held}"
    );
}

#[test]
fn absent() {
    let seat = seated(DECLARED);
    let named = seat
        .path()
        .file_name()
        .map(|held| held.to_string_lossy().to_string())
        .expect("fixture name");
    std::fs::remove_file(seat.path().join("charts").join(&named).join("SKILL.md")).expect("gone");
    std::fs::write(seat.path().join("charts").join(&named).join("kept"), "held").expect("leaf");
    track(seat.path());
    let held = report(seat.path());
    assert!(held.contains("SKILL.md is not a tracked leaf"), "{held}");
}
