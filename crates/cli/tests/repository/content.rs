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
    inspect(
        &format!("[member.entry.lines]\n{rule}"),
        ".gitattributes",
        body,
    )
}

#[test]
fn fields() {
    let rule = "[[member.entry.fields]]\ndeny = ['packageManager']";
    for text in [
        r#"{"packageManager":null}"#,
        "{\n\"packageManager\": \"pnpm@11.13.0\"\n}",
    ] {
        let held = inspect(rule, "package.json", Some(text.as_bytes()));
        assert!(held.contains("unapproved field"), "{held}");
    }
    let held = inspect(
        rule,
        "package.json",
        Some(br#"{"description":"packageManager"}"#),
    );
    assert!(!held.contains("unapproved field"), "{held}");
    let held = inspect(rule, "package.json", Some(b"{invalid-private-value"));
    assert!(held.contains("invalid governed JSON"), "{held}");
    assert!(!held.contains("invalid-private-value"), "{held}");
    let held = inspect(
        "[[member.entry.fields]]\ndeny = 7",
        "package.json",
        Some(b"{}"),
    );
    assert!(held.contains("invalid member fields"), "{held}");
}

fn inspect(rule: &str, name: &str, body: Option<&[u8]>) -> String {
    let root = tempfile::tempdir().expect("repository");
    let rules = format!("[member]\n[[member.entry]]\nname = \"content\"\n{rule}\n");
    let depot = super::support::overlay(&[("rules/atoms/seat.toml", &rules)]);
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(["init", "-q"])
            .status()
            .expect("git")
            .success()
    );
    std::fs::write(root.path().join("plumb.toml"), format!("[[layout.file]]\nname = [\"{name}\"]\nrule = [\"rule://seat/content\"]\nnote = \"content\"\n")).expect("layout");
    if let Some(body) = body {
        std::fs::write(root.path().join(name), body).expect("content");
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

#[test]
fn probe() {
    let output = Command::new("git").arg("--version").output().expect("git");
    let version = String::from_utf8(output.stdout).expect("version");
    let rule = format!(
        "[[member.entry.probe]]\nargv = ['git', '--version']\nstdout = {:?}",
        version
    );
    let held = inspect(&rule, "Cargo.toml", Some(b""));
    assert!(!held.contains("expected stdout"), "{held}");
    let held = inspect(
        "[[member.entry.probe]]\nargv = ['git', '--version']\nstdout = 'wrong'",
        "Cargo.toml",
        Some(b""),
    );
    assert!(held.contains("expected stdout"), "{held}");
    let missing = "[[member.entry.probe]]\nargv = ['plumb-probe-does-not-exist']\nstdout = ''";
    let held = inspect(missing, "Cargo.toml", Some(b""));
    assert!(held.contains("cannot resolve tool"), "{held}");
    let held = inspect(missing, "Cargo.toml", None);
    assert!(!held.contains("cannot resolve tool"), "{held}");
    let held = inspect(
        &format!("{rule}\nplatform = ['unavailable']"),
        "Cargo.toml",
        Some(b""),
    );
    assert!(held.contains("no probe rule for platform"), "{held}");
    let held = inspect(&format!("{rule}\n{rule}"), "Cargo.toml", Some(b""));
    assert!(held.contains("conflicting probe rules"), "{held}");
}
