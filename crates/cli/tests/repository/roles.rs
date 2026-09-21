use std::path::Path;
use std::process::Command;

#[test]
fn whitelist() {
    let held = report("allow = [\"cli\", \"api\", \"lib\"]");
    for name in ["cli", "api", "lib"] {
        assert!(
            !held.contains(&format!("crates/{name} is outside")),
            "{held}"
        );
    }
    assert!(held.contains("crates/other is outside"), "{held}");
}

#[test]
fn blacklist() {
    let held = report("allow = [\"cli\", \"api\", \"lib\"]\ndeny = [\"api\"]");
    assert!(held.contains("crates/api is outside"), "{held}");
    assert!(!held.contains("crates/cli is outside"), "{held}");
}

#[test]
fn empty() {
    let held = report("allow = []");
    assert!(held.contains("crates/cli is outside"), "{held}");
    let held = report("deny = [\"other\"]");
    assert!(!held.contains("crates/cli is outside"), "{held}");
    assert!(held.contains("crates/other is outside"), "{held}");
}

#[test]
fn malformed() {
    for rule in ["allow = [7]", "allow = \"cli\"", "deny = [\"../cli\"]"] {
        let held = report(rule);
        assert!(
            held.contains("member allow must") || held.contains("invalid name"),
            "{held}"
        );
    }
}

fn report(rule: &str) -> String {
    let root = tempfile::tempdir().expect("repository");
    let rules = format!("[member]\n[[member.entry]]\nname = \"roles\"\n{rule}\n");
    let depot = super::support::overlay(&[("rules/seat.toml", &rules)]);
    git(root.path(), &["init", "-q"]);
    std::fs::write(
        root.path().join("plumb.toml"),
        "[[layout.seat]]\npath = \"crates/*\"\nrule = [\"rule://seat/roles\"]\nnote = \"roles\"\n",
    )
    .expect("layout");
    for name in ["cli", "api", "lib", "other"] {
        let path = root.path().join("crates").join(name);
        std::fs::create_dir_all(&path).expect("role");
        std::fs::write(path.join("kept"), "fixture").expect("member");
    }
    git(root.path(), &["add", "."]);
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", depot.path())
        .arg("doctor")
        .arg(root.path())
        .output()
        .expect("doctor");
    let shown = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        shown.contains("crates"),
        "{shown}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    shown
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .expect("git")
            .success()
    );
}
