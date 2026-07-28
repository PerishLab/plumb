use std::collections::BTreeSet;
use std::path::Path;

pub fn actions(root: &Path) -> BTreeSet<String> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return BTreeSet::new();
    };
    entries
        .flatten()
        .filter(|entry| {
            entry.path().is_dir()
                && (entry.path().join("action.yml").is_file()
                    || entry.path().join("action.yaml").is_file())
        })
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect()
}

pub fn tests(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    collect(root, &root.join(".runseal"), &mut found);
    found.sort();
    found
}

fn collect(root: &Path, at: &Path, found: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(at) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, found);
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if (name.ends_with(".test.ts") || name.ends_with(".test.tsx"))
            && let Ok(relative) = path.strip_prefix(root)
        {
            found.push(relative.to_string_lossy().to_string());
        }
    }
}
