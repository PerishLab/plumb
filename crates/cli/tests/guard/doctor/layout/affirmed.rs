use super::{DECLARED, home, report, seated, track};
use std::process::Command;

#[test]
fn affirmed() {
    let seat = seated(&DECLARED.replace(
        "rule = [\"rule://seat/named-after-repository\", \"rule://seat/wayfinder\"]",
        "rule = [\"rule://seat/affirmed\"]",
    ));
    let held = report(seat.path());
    assert!(held.contains("was affirmed against a different"), "{held}");
    assert!(
        held.contains("plumb cookbook structure.seat-affirmed"),
        "{held}"
    );
}

#[test]
fn recorded() {
    let seat = seated(&DECLARED.replace(
        "rule = [\"rule://seat/named-after-repository\", \"rule://seat/wayfinder\"]",
        "rule = [\"rule://seat/affirmed\"]",
    ));
    let done = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["affirm", seat.path().to_str().expect("utf8"), "--write"])
        .env("PLUMB_HOME", home())
        .output()
        .expect("plumb");
    assert!(
        done.status.success(),
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
    track(seat.path());
    let held = report(seat.path());
    assert!(!held.contains("was affirmed against a different"), "{held}");
}

#[test]
fn layered() {
    let seat = seated(DECLARED);
    std::fs::write(seat.path().join("LICENSE"), "held\n").expect("license");
    track(seat.path());
    let layered = report(seat.path());
    assert!(!layered.contains("sits in no declared seat"), "{layered}");
    assert!(
        layered.contains("plumb.toml overrides the Plumb default path charts/*"),
        "{layered}"
    );
    assert!(!layered.contains("default name LICENSE"), "{layered}");
}

#[test]
fn wording() {
    let seat = seated(&DECLARED.replace(
        "rule = [\"rule://seat/named-after-repository\", \"rule://seat/wayfinder\"]",
        "rule = [\"rule://seat/affirmed\"]",
    ));
    let done = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["affirm", seat.path().to_str().expect("utf8"), "--write"])
        .env("PLUMB_HOME", home())
        .output()
        .expect("plumb");
    assert!(done.status.success());
    let manifest = seat.path().join("plumb.toml");
    let held = std::fs::read_to_string(&manifest).expect("manifest");
    std::fs::write(
        &manifest,
        held.replace("the governance pair", "the pair that governs"),
    )
    .expect("reworded");
    track(seat.path());
    let reworded = report(seat.path());
    assert!(
        !reworded.contains("was affirmed against a different"),
        "{reworded}"
    );

    let held = std::fs::read_to_string(&manifest).expect("manifest");
    std::fs::write(
        &manifest,
        format!("{held}\n[[layout.seat]]\npath = \"docs\"\n"),
    )
    .expect("reshaped");
    track(seat.path());
    let reshaped = report(seat.path());
    assert!(
        reshaped.contains("was affirmed against a different"),
        "{reshaped}"
    );
}

#[test]
fn unstaged() {
    let seat = seated(&DECLARED.replace(
        "rule = [\"rule://seat/named-after-repository\", \"rule://seat/wayfinder\"]",
        "rule = [\"rule://seat/affirmed\"]",
    ));
    let removed = seat.path().join("ectropy.toml");
    std::fs::write(&removed, "policy = 1\n").expect("ectropy");
    track(seat.path());
    std::fs::remove_file(&removed).expect("delete");

    for args in [
        vec!["affirm", seat.path().to_str().expect("utf8")],
        vec!["affirm", seat.path().to_str().expect("utf8"), "--write"],
    ] {
        let done = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .args(&args)
            .env("PLUMB_HOME", home())
            .output()
            .expect("plumb");
        assert!(!done.status.success(), "unstaged deletion must fail closed");
        let stderr = String::from_utf8_lossy(&done.stderr);
        assert!(stderr.contains("ectropy.toml"), "{stderr}");
        assert!(stderr.contains("git rm ectropy.toml"), "{stderr}");
        assert!(stderr.contains("git add -u"), "{stderr}");
        assert!(stderr.contains("restore"), "{stderr}");
        assert!(stderr.contains("before running `plumb affirm`"), "{stderr}");
        assert!(!stderr.contains("os error 2"), "{stderr}");
        assert!(!seat.path().join(".plumb/affirmed.toml").exists());
    }

    let staged = Command::new("git")
        .args(["-C", seat.path().to_str().expect("utf8"), "add", "-u"])
        .status()
        .expect("git");
    assert!(staged.success());

    let done = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["affirm", seat.path().to_str().expect("utf8"), "--write"])
        .env("PLUMB_HOME", home())
        .output()
        .expect("plumb");
    assert!(
        done.status.success(),
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
    assert!(seat.path().join(".plumb/affirmed.toml").is_file());
}

#[test]
fn multiple() {
    let seat = seated(&DECLARED.replace(
        "rule = [\"rule://seat/named-after-repository\", \"rule://seat/wayfinder\"]",
        "rule = [\"rule://seat/affirmed\"]",
    ));
    std::fs::write(seat.path().join("ectropy.toml"), "a = 1\n").expect("ectropy");
    std::fs::write(seat.path().join("runseal.toml"), "b = 2\n").expect("runseal");
    track(seat.path());
    std::fs::remove_file(seat.path().join("ectropy.toml")).expect("delete ectropy");
    std::fs::remove_file(seat.path().join("runseal.toml")).expect("delete runseal");

    let done = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["affirm", seat.path().to_str().expect("utf8"), "--write"])
        .env("PLUMB_HOME", home())
        .output()
        .expect("plumb");
    assert!(
        !done.status.success(),
        "unstaged deletions must fail closed"
    );
    let stderr = String::from_utf8_lossy(&done.stderr);
    assert!(stderr.contains("ectropy.toml"), "{stderr}");
    assert!(stderr.contains("runseal.toml"), "{stderr}");
    assert!(stderr.contains("git rm"), "{stderr}");
    assert!(stderr.contains("git add -u"), "{stderr}");
    assert!(stderr.contains("restore"), "{stderr}");
    assert!(stderr.contains("before running `plumb affirm`"), "{stderr}");
    assert!(!stderr.contains("os error 2"), "{stderr}");
    assert!(!seat.path().join(".plumb/affirmed.toml").exists());

    let restored = Command::new("git")
        .args([
            "-C",
            seat.path().to_str().expect("utf8"),
            "checkout",
            "--",
            "ectropy.toml",
            "runseal.toml",
        ])
        .status()
        .expect("git");
    assert!(restored.success());

    let done = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["affirm", seat.path().to_str().expect("utf8"), "--write"])
        .env("PLUMB_HOME", home())
        .output()
        .expect("plumb");
    assert!(
        done.status.success(),
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
    assert!(seat.path().join(".plumb/affirmed.toml").is_file());
}
