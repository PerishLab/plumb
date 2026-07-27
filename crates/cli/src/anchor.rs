use std::path::{Path, PathBuf};

const DERIVE: &str = concat!("der", "ive(");
const CASCADE: &str = concat!("Casc", "ade");

pub struct Anchor<'a>(pub &'a Path);

impl Anchor<'_> {
    fn seats(&self) -> Vec<(String, PathBuf)> {
        let mut held = vec![
            (".".to_string(), self.0.to_path_buf()),
            ("app".to_string(), self.0.join("app")),
        ];
        let Ok(entries) = std::fs::read_dir(self.0.join("crates")) else {
            return held;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            held.push((format!("crates/{name}"), entry.path()));
        }
        held
    }

    pub fn names(&self) -> Vec<(String, String)> {
        let mut held = Vec::new();
        for (seat, dir) in self.seats() {
            if let Some(name) = name(&dir) {
                held.push((seat, name));
            }
        }
        held
    }

    pub fn derives(&self) -> Vec<String> {
        let mut held = Vec::new();
        for (seat, dir) in self.seats() {
            let Ok(manifest) = std::fs::read_to_string(dir.join("Cargo.toml")) else {
                continue;
            };
            if manifest.contains("proc-macro = true") {
                continue;
            }
            if holds(&dir.join("src")) {
                held.push(seat);
            }
        }
        held
    }

    pub fn entries(&self) -> Vec<String> {
        self.seats()
            .into_iter()
            .filter(|(_, dir)| entry(dir))
            .map(|(seat, _)| seat)
            .collect()
    }

    pub fn repo(&self) -> Option<String> {
        let root = std::fs::canonicalize(self.0).ok()?;
        home(&root)
            .unwrap_or(root)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
    }
}

fn home(root: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(root.join(".git")).ok()?;
    let gitdir = root.join(text.strip_prefix("gitdir:")?.trim());
    let common = std::fs::read_to_string(gitdir.join("commondir")).ok()?;
    std::fs::canonicalize(gitdir.join(common.trim()))
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

fn name(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("Cargo.toml")).ok()?;
    let doc = text.parse::<toml::Table>().ok()?;
    doc.get("package")
        .and_then(|value| value.get("name"))
        .and_then(toml::Value::as_str)
        .map(str::to_string)
}

fn entry(dir: &Path) -> bool {
    if dir.join("src/main.rs").is_file() || dir.join("src/bin").is_dir() {
        return true;
    }
    let Ok(text) = std::fs::read_to_string(dir.join("Cargo.toml")) else {
        return false;
    };
    let Ok(doc) = text.parse::<toml::Table>() else {
        return false;
    };
    doc.get("bin").and_then(toml::Value::as_array).is_some()
}

fn holds(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let path = entry.path();
        if path.is_dir() {
            return holds(&path);
        }
        path.extension().is_some_and(|ext| ext == "rs") && marked(&path)
    })
}

fn marked(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    text.lines()
        .any(|line| line.contains(DERIVE) && line.contains(CASCADE))
}
