use super::codec::{Binding, Codec, Origin};
use serde_json::Value;
use std::{fs, path::PathBuf};

fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/identity")
}

fn text(value: &Value, field: &str) -> String {
    value[field].as_str().unwrap().to_owned()
}

#[test]
fn release_identity_fixtures() {
    let manifest: Value =
        serde_json::from_slice(&fs::read(directory().join("manifest.json")).unwrap()).unwrap();
    let cases = manifest.as_object().unwrap();
    assert!(!cases.is_empty());
    for (name, expect) in cases {
        let bytes = fs::read(directory().join(name)).unwrap();
        let decoded = Codec(&bytes).decode();
        if expect.get("refused").is_some() {
            assert!(decoded.is_err(), "{name} must be refused");
            continue;
        }
        let (origin, binding) = decoded.unwrap_or_else(|error| panic!("{name}: {error}"));
        let held = &expect["origin"];
        assert_eq!(
            origin,
            Origin {
                prefix: text(held, "prefix"),
                commit: text(held, "commit"),
                target: text(held, "target"),
            },
            "{name}"
        );
        let wanted = match &expect["binding"] {
            Value::Null => None,
            held => Some(Binding {
                product: text(held, "product"),
                marker: text(held, "marker"),
                digest: text(held, "digest"),
                commit: text(held, "commit"),
                workload: text(held, "workload"),
            }),
        };
        assert_eq!(binding, wanted, "{name}");
        if let Some(wanted) = wanted {
            let unbound = fs::read(directory().join("unbound.region")).unwrap();
            assert_eq!(Codec(&unbound).encode(&wanted).unwrap(), bytes, "{name}");
        }
    }
}
