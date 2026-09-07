use std::process::Command;

#[test]
fn whitelist() {
    let rules = "allow = [\"* text=auto eol=lf\"]\nrequired = [\"* text=auto eol=lf\"]";
    let held = report(rules, Some(b"# policy\n* text=auto eol=lf\n\n"));
    assert!(!held.contains("unapproved content"), "{held}");
    assert!(!held.contains("missing required content"), "{held}");
    let held = report(rules, Some(b".runseal/** text eol=lf\n"));
    assert!(held.contains("unapproved content"), "{held}");
    assert!(held.contains("missing required content"), "{held}");
}

#[test]
fn blacklist() {
    let held = report("allow = [\"held\"]\ndeny = [\"held\"]", Some(b"held\n"));
    assert!(held.contains("unapproved content"), "{held}");
}

#[test]
fn missing() {
    let held = report("required = [\"held\"]", None);
    assert!(held.contains("missing its governed content"), "{held}");
}

#[test]
fn unreadable() {
    let held = report("required = [\"held\"]", Some(&[255]));
    assert!(held.contains("content is not UTF-8"), "{held}");
    let held = report("allow = 7", Some(b"held\n"));
    assert!(held.contains("invalid member lines"), "{held}");
}

fn report(rule: &str, body: Option<&[u8]>) -> String {
    let root = tempfile::tempdir().expect("repository");
    let rules =
        format!("[member]\n[[member.entry]]\nname = \"content\"\n[member.entry.lines]\n{rule}\n");
    let depot = super::support::depot(&[("rules/seat.toml", &rules)]);
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(["init", "-q"])
            .status()
            .expect("git")
            .success()
    );
    std::fs::write(root.path().join("plumb.toml"), "[[layout.file]]\nname = [\".gitattributes\"]\nrule = [\"rule://seat/content\"]\nnote = \"content\"\n").expect("layout");
    if let Some(body) = body {
        std::fs::write(root.path().join(".gitattributes"), body).expect("content");
    }
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(["add", "."])
            .status()
            .expect("git")
            .success()
    );
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .env("PLUMB_HOME", depot.path())
        .arg("doctor")
        .arg(root.path())
        .output()
        .expect("doctor");
    let shown = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        shown.contains("plumb doctor"),
        "{shown}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    shown
}
