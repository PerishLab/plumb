use super::{pilot::*, successor::Stage};
use serde_json::json;

#[test]
fn stages() {
    let mut pilot = Pilot::new();
    pilot.inspect(NEXT, Stage::Open(SOURCE)).unwrap();
    pilot.line();
    pilot.inspect(RC, Stage::Stamp(SOURCE, RC)).unwrap();
    pilot.marked(RC);
    pilot.inspect(RC, Stage::Dispatch(SOURCE)).unwrap();
    let promotion = format!("{NEXT}\n\npromotes {RC}");
    assert!(
        pilot
            .inspect(NEXT, Stage::Stamp(SOURCE, &promotion))
            .is_err()
    );
    pilot.shipped();
    pilot
        .inspect(NEXT, Stage::Stamp(SOURCE, &promotion))
        .unwrap();
    pilot.marked(NEXT);
    pilot.inspect(NEXT, Stage::Dispatch(SOURCE)).unwrap();
    for (key, value) in [
        ("state", "incomplete"),
        ("marker", "v9.9.9-rc.1"),
        ("commit", BASE),
        ("tree", BASE),
        ("repository", "Example/elsewhere"),
    ] {
        pilot.shipped();
        pilot.records.get_mut(&receipt(RC)).unwrap()[key] = json!(value);
        assert!(
            pilot.inspect(NEXT, Stage::Dispatch(SOURCE)).is_err(),
            "{key}"
        );
    }
}

#[test]
fn changes() {
    let cases: [fn(&mut Pilot); 14] = [
        |p| p.blob = "a".repeat(40),
        |p| {
            p.records
                .get_mut(&native("repositories/1388431417/branches/main"))
                .unwrap()["protected"] = json!(false);
        },
        |p| {
            p.records
                .get_mut(&native("repositories/1388431417"))
                .unwrap()["id"] = json!(17);
        },
        |p| {
            p.records.get_mut(&native("repos/Example/seat")).unwrap()["id"] = json!(18);
        },
        |p| {
            p.records.remove(&receipt(OLD));
        },
        |p| {
            p.records.get_mut(&receipt(OLD)).unwrap()["state"] = json!("complete");
        },
        |p| {
            p.records.get_mut(&channel()).unwrap()["releaseVersion"] = json!(OLD);
        },
        |p| {
            p.records.get_mut(&product("git/ref/heads/main")).unwrap()["object"]["sha"] =
                json!(BASE);
        },
        |p| {
            p.records
                .get_mut(&product(&format!("git/ref/tags/{OLD}")))
                .unwrap()["object"]["type"] = json!("commit");
        },
        |p| {
            p.records
                .get_mut(&product(&format!("git/commits/{SOURCE}")))
                .unwrap()["tree"]["sha"] = json!(BASE);
        },
        |p| p.owed.push("release.line-closed"),
        |p| p.owed.clear(),
        |p| p.listing.clear(),
        |p| {
            p.git(&[
                "remote",
                "set-url",
                "--push",
                "origin",
                "git@github.com:Example/other.git",
            ])
        },
    ];
    for (index, change) in cases.into_iter().enumerate() {
        let mut pilot = Pilot::new();
        change(&mut pilot);
        assert!(
            pilot.inspect(NEXT, Stage::Open(SOURCE)).is_err(),
            "case {index}"
        );
    }
}

#[test]
fn bindings() {
    for (path, key, value) in [
        ("predecessor", "tag", SOURCE),
        ("predecessor", "marker", "v1.2.2"),
        ("predecessor", "distribution_sha256", &"0".repeat(64)),
        ("successor", "commit", BASE),
        ("successor", "rc", "v1.2.4-beta.1"),
        ("successor", "marker", "v1.2.5"),
    ] {
        let mut p = Pilot::new();
        p.record[path][key] = json!(value);
        p.approve();
        assert!(p.inspect(NEXT, Stage::Open(SOURCE)).is_err(), "{key}");
    }
    let p = Pilot::new();
    for marker in ["v1.2.4-beta.1", "v1.2.4-rc.2", "v1.3.0"] {
        assert!(p.inspect(marker, Stage::Open(SOURCE)).is_err(), "{marker}");
    }
}

#[test]
fn withdrawal() {
    let mut p = Pilot::new();
    p.revoked = true;
    assert!(
        p.inspect(NEXT, Stage::Open(SOURCE))
            .unwrap_err()
            .contains("main changed")
    );
    let mut p = Pilot::new();
    p.record["extra"] = json!(true);
    p.approve();
    assert!(
        p.inspect(NEXT, Stage::Open(SOURCE))
            .unwrap_err()
            .contains("invalid decision")
    );
}

#[test]
fn destination() {
    let p = Pilot::new();
    p.git(&[
        "remote",
        "set-url",
        "--push",
        "origin",
        "git@github.com:Example/seat.git",
    ]);
    p.git(&[
        "config",
        "url.https://cache.example/.pushInsteadOf",
        "git@github.com:",
    ]);
    assert!(
        p.inspect(NEXT, Stage::Open(SOURCE))
            .unwrap_err()
            .contains("rewritten")
    );
}

#[test]
fn missing() {
    let mut p = Pilot::new();
    p.records
        .retain(|key, _| !key.contains("contents/governance/release-successors/"));
    assert!(
        p.inspect(NEXT, Stage::Open(SOURCE))
            .unwrap_err()
            .contains("unavailable")
    );
    let mut p = Pilot::new();
    p.line();
    p.marked(RC);
    p.shipped();
    p.marked(NEXT);
    p.records.remove(&product(&format!("git/ref/tags/{RC}")));
    assert!(
        p.inspect(NEXT, Stage::Dispatch(SOURCE))
            .unwrap_err()
            .contains("unavailable")
    );
}

#[test]
fn spent() {
    let mut p = Pilot::new();
    p.line();
    p.marked(RC);
    p.shipped();
    p.marked(NEXT);
    let mut finished = p.records[&receipt(RC)].clone();
    finished["marker"] = json!(NEXT);
    p.records.insert(receipt(NEXT), finished);
    assert!(
        p.inspect(NEXT, Stage::Dispatch(SOURCE))
            .unwrap_err()
            .contains("already completed")
    );
    p.records.get_mut(&receipt(NEXT)).unwrap()["state"] = json!("incomplete");
    p.inspect(NEXT, Stage::Dispatch(SOURCE)).unwrap();
    p.records.get_mut(&receipt(NEXT)).unwrap()["commit"] = json!(BASE);
    assert!(
        p.inspect(NEXT, Stage::Dispatch(SOURCE))
            .unwrap_err()
            .contains("receipt source differs")
    );
}
