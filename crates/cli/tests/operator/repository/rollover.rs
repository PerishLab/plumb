use super::{
    cargo::{Fixture, report},
    support,
};
use serde_json::Value;
use std::process::Output;

fn rollover(repeated: bool) -> Output {
    let fixture = Fixture::new(true, &["0.1.0", "0.2.0"]);
    let tools = fixture.tools.path();
    let next = tools.join("next");
    Fixture::offer(&next, &["0.1.0", "0.2.0"]);
    Fixture::offer(tools, &["0.1.0"]);
    if repeated {
        Fixture::offer(&tools.join("again"), &["0.1.0"]);
    }
    support::executable(
        &tools.join("plumb"),
        r#"#!/bin/sh
if [ "$1" = --version ]; then echo 'plumb fixture'; exit 0; fi
[ "$1" = doctor ] || exit 90
seat=$(dirname "$0")
for offer in next again; do
    if [ -f "$seat/$offer/index/keel" ]; then
        mv "$seat/$offer/index/keel" "$seat/$offer/index/keel-gate" "$seat/index/" || exit 91
        if [ -d "$seat/again" ]; then
            echo 'fixture Doctor saw a new stable' >&2
            exit 1
        fi
        exit 0
    fi
done
grep -q 'version = "0.2.0"' Cargo.lock
"#,
    );
    support::executable(&tools.join("ectropy"), "#!/bin/sh\necho clean\n");
    let config = fixture.root.path().join("plumb.toml");
    let text = std::fs::read_to_string(&config).unwrap();
    std::fs::write(
        config,
        format!("{text}ectropy = ['src']\nplumb = ['src']\n"),
    )
    .unwrap();
    fixture.git(&["add", "plumb.toml"]);
    fixture.git(&["commit", "-qm", "guard actions"]);
    let before = fixture.lock();
    let output = fixture.plumb("guard");
    assert_eq!(fixture.lock(), before);
    assert_eq!(fixture.git(&["status", "--porcelain"]), "");
    let log = String::from_utf8_lossy(&output.stderr);
    assert!(
        log.contains("guard.packages.rollover: cargo keel 0.1.0 -> 0.2.0"),
        "{log}"
    );
    assert!(log.contains("guard reused guard/ectropy"), "{log}");
    assert!(log.contains("guard restarted guard/plumb"), "{log}");
    output
}

#[test]
fn refresh() {
    let output = rollover(false);
    let log = String::from_utf8_lossy(&output.stderr);
    assert!(log.contains("guard guard/rust"), "{log}");
    assert!(log.contains("guard restarted guard/rust"), "{log}");
    let evidence = report(&output);
    let packages = evidence["guard"]["resolution"]["packages"]
        .as_array()
        .unwrap();
    assert_eq!(packages.len(), 2);
    assert!(packages.iter().all(|package| package["version"] == "0.2.0"));
}

#[test]
fn repeated() {
    let output = rollover(true);
    assert!(!output.status.success());
    let refusal: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(refusal["ok"], false);
    assert!(refusal.get("guard").is_none());
    let message = refusal["message"].as_str().unwrap();
    assert!(message.contains("guard.packages.rollover: cargo keel 0.2.0 -> 0.1.0"));
    assert!(message.contains("refused after one restart"));
}
