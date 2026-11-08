use plumb::guard::{Action, Descriptor, Runtime};
use plumb::packages::Resolution;

struct Fixture;

impl Fixture {
    fn runtime() -> Runtime {
        let guard = Descriptor {
            schema: plumb::guard::SCHEMA.into(),
            repository: "PerishLab/plumb".into(),
            tree: "b".repeat(40),
            plumb: format!("v0.138.0@{}", "c".repeat(40)),
            depot: "d".repeat(64),
            platform: "linux-x86_64".into(),
            actions: vec![Action {
                name: "guard/test".into(),
                input: "a".repeat(64),
                world: "e".repeat(64),
            }],
            resolution: None,
            digest: String::new(),
        }
        .resolved(Resolution {
            context: "ci-latest".into(),
            tree: "b".repeat(40),
            packages: vec![],
        })
        .unwrap();
        let commit = "f".repeat(40);
        let digest = Runtime::seal(&commit, &guard).unwrap();
        Runtime {
            schema: Runtime::SCHEMA.into(),
            root: "/fixture/source".into(),
            ok: true,
            strength: "full".into(),
            boundary: "head".into(),
            commit,
            guard,
            digest,
        }
    }
}

#[test]
fn legacy() {
    let held = Fixture::runtime();
    assert_eq!(
        held.guard.digest,
        "b8727631dd07dd79e7331ddc27cbdf2edcff4e02fd4fa33611bd76d2841e0bf1"
    );
    assert_eq!(
        held.digest,
        "9be542a1a4f976d8813a212dac3216d67e907f44346c98df85833e1d9d481c91"
    );
    let raw = serde_json::to_vec(&held).unwrap();
    Runtime::parse(&raw).unwrap();
}

#[test]
fn tampering() {
    let held = Fixture::runtime();
    for changed in 0..5 {
        let mut value = held.clone();
        match changed {
            0 => value.ok = false,
            1 => value.strength = "partial".into(),
            2 => value.commit = "a".repeat(40),
            3 => value.guard.actions[0].world = "0".repeat(64),
            _ => value.digest = "0".repeat(64),
        }
        assert!(value.validate().is_err());
    }
}

#[test]
fn worlds() {
    use plumb::semantics::admission::{Control, Controls, Point, Stable, World};
    use std::collections::BTreeMap;

    let guard = Fixture::runtime();
    let held = World {
        schema: World::SCHEMA.into(),
        source: Point {
            repository: "PerishLab/plumb".into(),
            marker: "v0.138.0".into(),
            commit: "a".repeat(40),
            tree: "b".repeat(40),
        },
        head: guard.commit.clone(),
        tree: "b".repeat(40),
        packages: vec![],
        controls: Controls {
            plumb: "plumb v0.138.0".into(),
            ectropy: "ectropy v0.51.0".into(),
            authority: Control {
                producer: guard.guard.plumb.clone(),
                depot: guard.guard.depot.clone(),
            },
            stable: BTreeMap::from([
                (
                    "plumb".into(),
                    Stable {
                        marker: "v0.138.0".into(),
                        commit: "c".repeat(40),
                    },
                ),
                (
                    "ectropy".into(),
                    Stable {
                        marker: "v0.51.0".into(),
                        commit: "e".repeat(40),
                    },
                ),
            ]),
        },
        domain: BTreeMap::from([
            ("node.version".into(), "24.18.0".into()),
            ("pnpm.version".into(), "11.28.5".into()),
            ("rust.version".into(), "1.96.1".into()),
        ]),
        guard,
    };
    World::parse(&serde_json::to_vec(&held).unwrap()).unwrap();
    let raw = serde_json::to_string(&held).unwrap();
    let ambiguous = raw.replace(
        "\"node.version\":\"24.18.0\"",
        "\"node.version\":\"invalid\",\"node.version\":\"24.18.0\"",
    );
    assert_ne!(raw, ambiguous);
    assert_eq!(
        World::parse(ambiguous.as_bytes()).unwrap_err().state,
        plumb::windtun::qualification::State::Refused
    );
    let mut reused = held.clone();
    reused.head = "a".repeat(40);
    reused.validate().unwrap();
    let mut missing = held.clone();
    missing.guard.guard.resolution = None;
    missing.guard.guard.digest =
        "d27fbee012bb9959f4daa46e3dd06e67ff31c6116432b40d4afcb6016f44ac5f".into();
    missing.guard.digest = Runtime::seal(&missing.guard.commit, &missing.guard.guard).unwrap();
    missing.guard.validate().unwrap();
    let raw = serde_json::to_vec(&missing).unwrap();
    assert_eq!(
        World::parse(&raw).unwrap_err().state,
        plumb::windtun::qualification::State::Unknown
    );
    for changed in 0..7 {
        let mut value = held.clone();
        match changed {
            0 => {
                value.domain.remove("rust.version");
            }
            1 => value.controls.plumb = "plumb v0.139.0".into(),
            2 => value.controls.authority.depot = "0".repeat(64),
            3 => value.source.repository = "PerishLab/concord".into(),
            4 => value.tree = "0".repeat(40),
            5 => value.guard.guard.resolution = None,
            _ => value.controls.stable.get_mut("plumb").unwrap().commit = "0".repeat(40),
        }
        assert!(value.validate().is_err(), "{changed}");
    }
}
