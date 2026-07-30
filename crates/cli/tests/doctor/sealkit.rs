use super::run;
use std::path::Path;

pub(super) fn write(root: &Path, requirement: &str, resolution: &str) {
    lock(root, requirement, requirement, resolution);
}

fn lock(root: &Path, requirement: &str, lock_requirement: &str, resolution: &str) {
    std::fs::create_dir_all(root.join(".runseal")).expect("runseal fixture should be made");
    let specifier = if requirement == "*" {
        "jsr:@perish/sealkit".to_string()
    } else {
        format!("jsr:@perish/sealkit@{requirement}")
    };
    std::fs::write(
        root.join(".runseal/deno.json"),
        serde_json::json!({
            "imports": {
                "@perish/sealkit": specifier,
            },
        })
        .to_string(),
    )
    .expect("deno config should be written");
    std::fs::write(
        root.join(".runseal/deno.lock"),
        serde_json::json!({
            "version": "5",
            "specifiers": {
                format!("jsr:@perish/sealkit@{lock_requirement}"): resolution,
            },
            "workspace": {
                "dependencies": [
                    format!("jsr:@perish/sealkit@{lock_requirement}"),
                ],
            },
        })
        .to_string(),
    )
    .expect("deno lock should be written");
}

fn doctor(root: &Path) -> String {
    run(&["doctor", root.to_str().expect("path should be utf8")])
}

#[test]
fn lines() {
    let fixture = tempfile::tempdir().expect("fixture should be made");

    write(fixture.path(), "^0.2.1", "0.2.1");
    let legacy = doctor(fixture.path());
    assert!(legacy.contains("sealkit  ^0.2.1 -> 0.2.1"), "{legacy}");
    assert!(
        !legacy.contains("unsupported Sealkit requirement"),
        "{legacy}"
    );

    write(fixture.path(), "^0.3.1", "0.3.1");
    let floor = doctor(fixture.path());
    assert!(floor.contains("sealkit  ^0.3.1 -> 0.3.1"), "{floor}");
    assert!(
        !floor.contains("unsupported Sealkit requirement"),
        "{floor}"
    );
    assert!(!floor.contains("below supported Sealkit floor"), "{floor}");

    write(fixture.path(), "^0.3.1", "0.3.2");
    let patch = doctor(fixture.path());
    assert!(!patch.contains("unsupported Sealkit resolution"), "{patch}");
}

#[test]
fn canonical() {
    let fixture = tempfile::tempdir().expect("fixture should be made");

    lock(fixture.path(), "^0.3.1", "~0.3.1", "0.3.1");
    let normalized = doctor(fixture.path());
    assert!(
        normalized.contains("sealkit  ^0.3.1 -> 0.3.1"),
        "{normalized}"
    );
    assert!(normalized.contains("0 blind"), "{normalized}");
}

#[test]
fn ambiguous() {
    let fixture = tempfile::tempdir().expect("fixture should be made");
    lock(fixture.path(), "^0.3.1", "~0.3.1", "0.3.1");
    std::fs::write(
        fixture.path().join(".runseal/deno.lock"),
        serde_json::json!({
            "version": "5",
            "specifiers": {
                "jsr:@perish/sealkit@~0.3.1": "0.3.1",
                "jsr:@perish/sealkit@>=0.3.1, <0.4.0": "0.3.2",
            },
            "workspace": {
                "dependencies": [
                    "jsr:@perish/sealkit@~0.3.1",
                    "jsr:@perish/sealkit@>=0.3.1, <0.4.0",
                ],
            },
        })
        .to_string(),
    )
    .expect("ambiguous lock should be written");

    let ambiguous = doctor(fixture.path());
    assert!(
        ambiguous.contains("workspace Sealkit requirement is absent or ambiguous"),
        "{ambiguous}"
    );
    assert!(ambiguous.contains("1 blind"), "{ambiguous}");
}

#[test]
fn refusals() {
    let fixture = tempfile::tempdir().expect("fixture should be made");

    write(fixture.path(), "^0.1.14", "0.1.14");
    let requirement = doctor(fixture.path());
    assert!(
        requirement.contains("unsupported Sealkit requirement ^0.1.14; use ^0.2.1 or ^0.3.1"),
        "{requirement}"
    );

    write(fixture.path(), "^0.3.1", "0.3.0");
    let floor = doctor(fixture.path());
    assert!(
        floor.contains("Sealkit resolution 0.3.0 is below supported Sealkit floor 0.3.1"),
        "{floor}"
    );

    write(fixture.path(), "^0.2.1", "0.4.0");
    let outside = doctor(fixture.path());
    assert!(
        outside.contains("unsupported Sealkit resolution 0.4.0; transition admits 0.2 or ^0.3.1"),
        "{outside}"
    );
}

#[test]
fn blind() {
    let fixture = tempfile::tempdir().expect("fixture should be made");
    write(fixture.path(), "^0.2.1", "0.2.1");

    std::fs::remove_file(fixture.path().join(".runseal/deno.lock"))
        .expect("lock should be removed");
    let missing = doctor(fixture.path());
    assert!(
        missing.contains("cannot read Sealkit lock: .runseal/deno.lock is missing"),
        "{missing}"
    );
    assert!(missing.contains("1 blind"), "{missing}");

    std::fs::write(fixture.path().join(".runseal/deno.lock"), "{")
        .expect("broken lock should be written");
    let malformed = doctor(fixture.path());
    assert!(
        malformed.contains("cannot read Sealkit lock: invalid JSON"),
        "{malformed}"
    );
}
