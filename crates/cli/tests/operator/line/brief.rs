use super::order::Hub;
use std::path::Path;

const STEP: &str = "install probe v1.0.0, then run probe skill upgrade";

fn seat(home: &Path, version: Option<&str>) {
    let skills = home.join(".grok").join("skills");
    std::fs::create_dir_all(&skills).expect("grok seat");
    let Some(version) = version else {
        return;
    };
    let path = skills.join("probe");
    std::fs::create_dir_all(&path).expect("probe seat");
    std::fs::write(path.join("SKILL.md"), "# probe\n").expect("brief");
    std::fs::write(
        path.join("metadata.json"),
        format!(r#"{{"schema":1,"name":"probe","version":"{version}","keeper":"probe"}}"#),
    )
    .expect("mark");
}

fn skilled() -> Hub {
    let hub = Hub::new();
    let manifest = hub.work().join("plumb.toml");
    let declared = std::fs::read_to_string(&manifest).expect("manifest");
    std::fs::write(&manifest, format!("{declared}skill = true\n")).expect("skill");
    hub
}

fn closed(home: &Path) -> String {
    let hub = skilled();
    hub.line("v1.0.0");
    hub.mark("v1.0.0");
    let output = hub
        .command(&["release", "close", "--version", "v1.0.0"])
        .env("HOME", home)
        .output()
        .expect("plumb");
    let said = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        output.status.success(),
        "close: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(said.starts_with("closed release/v1.0.0 at "), "{said}");
    said
}

#[test]
fn lagging() {
    let home = tempfile::tempdir().expect("home");
    seat(home.path(), Some("v0.9.0"));
    let said = closed(home.path());
    let path = home.path().join(".grok/skills/probe");
    assert!(
        said.contains("observed on this machine, not owed:"),
        "{said}"
    );
    assert!(
        said.contains(&format!(
            "grok seat {} holds the v0.9.0 brief, below v1.0.0: {STEP}",
            path.display()
        )),
        "{said}"
    );
}

#[test]
fn missing() {
    let home = tempfile::tempdir().expect("home");
    seat(home.path(), None);
    let said = closed(home.path());
    assert!(said.contains(&format!("holds no brief: {STEP}")), "{said}");
}

#[test]
fn quiet() {
    let current = tempfile::tempdir().expect("home");
    seat(current.path(), Some("v1.0.0"));
    let bare = tempfile::tempdir().expect("home");
    for home in [current.path(), bare.path()] {
        let said = closed(home);
        assert_eq!(said.lines().count(), 1, "{said}");
        assert!(!said.contains("observed"), "{said}");
    }
}

fn planned(hub: &Hub, home: &Path, args: &[&str], step: &str) {
    let output = hub.command(args).env("HOME", home).output().expect("plumb");
    let plan = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(plan.contains(step), "{args:?}: {plan}");
    assert!(!plan.contains("observed"), "{args:?}: {plan}");
}

#[test]
fn ungated() {
    let home = tempfile::tempdir().expect("home");
    seat(home.path(), Some("v0.9.0"));
    let hub = skilled();
    hub.mark("v1.1.0-rc.1");
    planned(
        &hub,
        home.path(),
        &["release", "open", "--version", "v1.1.0", "--dry-run"],
        "refs/heads/release/v1.1.0",
    );
    hub.line("v1.1.0");
    planned(
        &hub,
        home.path(),
        &["release", "stamp", "--version", "v1.1.0-rc.2", "--dry-run"],
        "git tag -a v1.1.0-rc.2",
    );
    planned(
        &hub,
        home.path(),
        &["ship", "dispatch", "--marker", "v1.1.0-rc.1", "--dry-run"],
        "marker=v1.1.0-rc.1",
    );
}
