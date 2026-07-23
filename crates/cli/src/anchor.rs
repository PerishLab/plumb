use std::path::{Path, PathBuf};

const DERIVE: &str = concat!("der", "ive(");
const CASCADE: &str = concat!("Casc", "ade");

fn seats(root: &Path) -> Vec<(String, PathBuf)> {
    let mut held = vec![
        (".".to_string(), root.to_path_buf()),
        ("app".to_string(), root.join("app")),
    ];
    let Ok(entries) = std::fs::read_dir(root.join("crates")) else {
        return held;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        held.push((format!("crates/{name}"), entry.path()));
    }
    held
}

fn crate_name(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("Cargo.toml")).ok()?;
    let doc = text.parse::<toml::Table>().ok()?;
    doc.get("package")
        .and_then(|value| value.get("name"))
        .and_then(toml::Value::as_str)
        .map(str::to_string)
}

pub fn crate_names(root: &Path) -> Vec<(String, String)> {
    let mut held = Vec::new();
    for (seat, dir) in seats(root) {
        if let Some(name) = crate_name(&dir) {
            held.push((seat, name));
        }
    }
    held
}

fn holds_cascade(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        if path.is_dir() {
            return holds_cascade(&path);
        }
        path.extension().is_some_and(|ext| ext == "rs") && derives(&path)
    })
}

fn derives(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    text.lines()
        .any(|line| line.contains(DERIVE) && line.contains(CASCADE))
}

pub fn cascade_seats(root: &Path) -> Vec<String> {
    let mut held = Vec::new();
    for (seat, dir) in seats(root) {
        let Ok(manifest) = std::fs::read_to_string(dir.join("Cargo.toml")) else {
            continue;
        };
        if manifest.contains("proc-macro = true") {
            continue;
        }
        if holds_cascade(&dir.join("src")) {
            held.push(seat);
        }
    }
    held
}

pub fn repo(root: &Path) -> Option<String> {
    std::fs::canonicalize(root)
        .ok()?
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
}
