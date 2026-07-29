use std::path::{Path, PathBuf};

const TONGUES: [&str; 2] = ["en", "zh"];
const LEAVES: [&str; 2] = ["INDEX.md", "MIGRATION.md"];

pub fn stamped(version: &str) -> String {
    match version.strip_prefix('v') {
        Some(_) => version.to_string(),
        None => format!("v{version}"),
    }
}

pub fn seat(root: &Path, version: &str) -> PathBuf {
    root.join("docs/CHANGELOG").join(stamped(version))
}

pub fn read(root: &Path, version: &str) -> Vec<String> {
    let home = seat(root, version);
    let mut found = Vec::new();
    for tongue in TONGUES {
        for leaf in LEAVES {
            if let Some(note) = judged(&home, tongue, leaf) {
                found.push(note);
            }
        }
    }
    found
}

fn judged(home: &Path, tongue: &str, leaf: &str) -> Option<String> {
    let shown = format!("{tongue}/{leaf}");
    match std::fs::read_to_string(home.join(tongue).join(leaf)) {
        Ok(text) if !text.trim().is_empty() => None,
        Ok(_) => Some(format!("{shown} is empty")),
        Err(_) => Some(format!("{shown} is missing")),
    }
}
