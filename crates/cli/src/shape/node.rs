use serde_json::Value as Json;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub fn read(root: &Path) -> Vec<(String, String)> {
    let mut found = BTreeSet::new();
    for path in manifests(root) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(doc) = serde_json::from_str::<Json>(&text) else {
            continue;
        };
        let seat = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        for key in [
            "dependencies",
            "devDependencies",
            "peerDependencies",
            "optionalDependencies",
        ] {
            let Some(map) = doc.get(key).and_then(Json::as_object) else {
                continue;
            };
            for name in map.keys() {
                found.insert((seat.clone(), name.clone()));
            }
        }
    }
    found.into_iter().collect()
}

fn manifests(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let rootmanifest = root.join("package.json");
    if rootmanifest.is_file() {
        found.push(rootmanifest);
    }
    for seat in ["apps", "packages"] {
        let Ok(entries) = std::fs::read_dir(root.join(seat)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path().join("package.json");
            if path.is_file() {
                found.push(path);
            }
        }
    }
    found
}
