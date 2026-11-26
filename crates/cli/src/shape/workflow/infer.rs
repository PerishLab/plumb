use super::Key;

const SEATS: [&str; 7] = [
    "app/",
    "apps/",
    "crates/",
    "lib/",
    "packages/",
    "tests/",
    "tools/",
];

pub fn governed(manifest: Option<&str>, paths: &[&str]) -> bool {
    if paths.contains(&"ectropy.toml") {
        return true;
    }
    let Some(text) = manifest else {
        return false;
    };
    let Ok(doc) = text.parse::<toml::Table>() else {
        return false;
    };
    if doc.contains_key("ectropy") || doc.contains_key("runseal") {
        return true;
    }
    if paths.iter().any(|path| {
        matches!(*path, "Cargo.toml" | "pnpm-lock.yaml" | "runseal.toml")
            || path.starts_with(".runseal/")
            || SEATS.iter().any(|seat| path.starts_with(seat))
    }) {
        return true;
    }
    scans(&doc)
}

pub fn include(key: &mut Key, plumb: bool, ectropy: bool) {
    if key.name() != "guard/ectropy" || key.paths.iter().any(|path| path == "*") {
        return;
    }
    for (present, path) in [(plumb, "plumb.toml"), (ectropy, "ectropy.toml")] {
        if present && !key.paths.iter().any(|held| held == path) {
            key.paths.push(path.to_string());
        }
    }
}

fn scans(doc: &toml::Table) -> bool {
    doc.get("layout")
        .and_then(|layout| layout.get("seat"))
        .and_then(toml::Value::as_array)
        .is_some_and(|seats| {
            seats.iter().any(|seat| {
                seat.get("kind").and_then(toml::Value::as_str) != Some("retired")
                    && seat
                        .get("scan")
                        .and_then(toml::Value::as_array)
                        .is_some_and(|list| !list.is_empty())
            })
        })
}

#[cfg(test)]
mod tests {
    use super::super::{inferred, parse};
    use super::*;

    #[test]
    fn inference() {
        assert!(governed(None, &["ectropy.toml"]));
        assert!(!governed(None, &["Cargo.toml", "crates/a/src/lib.rs"]));
        assert!(governed(Some("[ectropy]\n"), &["plumb.toml"]));
        assert!(governed(Some("[runseal]\n"), &["plumb.toml"]));
        assert!(governed(
            Some("[identity]\nns='a'\n"),
            &["plumb.toml", "crates/a/src/lib.rs"]
        ));
        assert!(governed(
            Some("[[layout.seat]]\npath='custom'\nscan=['custom/**/*.rs']\n"),
            &["plumb.toml"]
        ));
        assert!(!governed(
            Some("[identity]\nns='hardwear'\n"),
            &["plumb.toml", "README.md", "cad/frame.step", "docs/notes.md"]
        ));
    }

    #[test]
    fn inputs() {
        let mut held = parse("[workflow.hash.guard]\nectropy = ['src']\n");
        let key = held.keys.first_mut().unwrap();
        include(key, true, false);
        assert!(key.paths.contains(&"plumb.toml".to_string()));
        assert!(!key.paths.contains(&"ectropy.toml".to_string()));
        include(key, true, true);
        assert!(key.paths.contains(&"ectropy.toml".to_string()));
        let mut wild = inferred(false, false, true, true);
        let key = wild
            .keys
            .iter_mut()
            .find(|k| k.name() == "guard/ectropy")
            .unwrap();
        include(key, true, true);
        assert_eq!(key.paths, vec!["*".to_string()]);
    }
}
