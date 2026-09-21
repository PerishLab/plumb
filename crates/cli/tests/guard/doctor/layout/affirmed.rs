use super::{DECLARED, SEAT, home, report, seated, track};
use std::process::Command;

#[test]
fn affirmed() {
    let seat = seated(&DECLARED.replace(
        "rule = [\"rule://seat/named-after-repository\", \"rule://seat/wayfinder\"]",
        "rule = [\"rule://seat/affirmed\"]",
    ));
    let held = report(seat.path());
    assert!(held.contains("was affirmed against a different"), "{held}");
    assert!(held.contains("plumb cookbook affirmed"), "{held}");
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
    let bare = report(seat.path());
    assert!(
        bare.contains("file LICENSE sits in no declared seat"),
        "{bare}"
    );

    let defaults = "[[layout.file]]\nname = [\"LICENSE\"]\n\n[[layout.seat]]\npath = \"charts/*\"\nanchor = [\"Chart.yaml\"]\n";
    let home = crate::support::depot(&[("rules/seat.toml", SEAT), ("rules/plumb.toml", defaults)]);
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["doctor", seat.path().to_str().expect("utf8")])
        .env("PLUMB_HOME", home.path())
        .output()
        .expect("plumb");
    let layered = String::from_utf8_lossy(&output.stdout);
    assert!(!layered.contains("sits in no declared seat"), "{layered}");
    assert!(!layered.contains("carries none of"), "{layered}");
    assert!(
        layered.contains("plumb.toml overrides the Depot default path charts/*"),
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
