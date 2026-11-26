use std::path::{Path, PathBuf};

const DERIVE: &str = concat!("der", "ive(");
const CASCADE: &str = concat!("Casc", "ade");
const PREFIX: &str = concat!("config::", "prefix");

pub struct Prefix {
    pub seat: String,
    pub file: String,
    pub value: Option<String>,
}

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
            if let Some(name) = Seat(&dir).name() {
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
            if Seat(&dir.join("src")).holds() {
                held.push(seat);
            }
        }
        held
    }

    pub fn entries(&self) -> Vec<String> {
        self.seats()
            .into_iter()
            .filter(|(_, dir)| Seat(dir).entry())
            .map(|(seat, _)| seat)
            .collect()
    }

    pub fn repo(&self) -> Option<String> {
        if let Some(name) = declared(self.0) {
            return Some(name);
        }
        let root = std::fs::canonicalize(self.0).ok()?;
        home(&root)
            .unwrap_or(root)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
    }

    pub fn ns(&self) -> Option<String> {
        identity(self.0).or_else(|| self.repo())
    }

    pub fn prefixes(&self) -> Vec<Prefix> {
        let mut found = Vec::new();
        for (seat, dir) in self.seats() {
            let Ok(manifest) = std::fs::read_to_string(dir.join("Cargo.toml")) else {
                continue;
            };
            if manifest.contains("proc-macro = true") {
                continue;
            }
            let mut files = Vec::new();
            Seat(&dir.join("src")).gather(&mut files);
            files.sort();
            for path in files {
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let relative = path
                    .strip_prefix(self.0)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                for value in extract(&text) {
                    found.push(Prefix {
                        seat: seat.clone(),
                        file: relative.clone(),
                        value,
                    });
                }
            }
        }
        found
    }
}

fn identity(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("plumb.toml")).ok()?;
    let doc = text.parse::<toml::Table>().ok()?;
    doc.get("identity")
        .and_then(|value| value.get("ns"))
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn declared(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let doc = text.parse::<toml::Table>().ok()?;
    let package = doc
        .get("workspace")
        .and_then(|value| value.get("package"))
        .or_else(|| doc.get("package"))?;
    let repository = package.get("repository")?.as_str()?;
    repository
        .trim_end_matches('/')
        .rsplit(['/', ':'])
        .next()
        .map(|name| name.strip_suffix(".git").unwrap_or(name).to_string())
        .filter(|name| !name.is_empty())
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

struct Seat<'a>(&'a Path);

impl Seat<'_> {
    fn name(&self) -> Option<String> {
        let text = std::fs::read_to_string(self.0.join("Cargo.toml")).ok()?;
        let doc = text.parse::<toml::Table>().ok()?;
        doc.get("package")
            .and_then(|value| value.get("name"))
            .and_then(toml::Value::as_str)
            .map(str::to_string)
    }

    fn entry(&self) -> bool {
        if self.0.join("src/main.rs").is_file() || self.0.join("src/bin").is_dir() {
            return true;
        }
        let Ok(text) = std::fs::read_to_string(self.0.join("Cargo.toml")) else {
            return false;
        };
        let Ok(doc) = text.parse::<toml::Table>() else {
            return false;
        };
        doc.get("bin").and_then(toml::Value::as_array).is_some()
    }

    fn holds(&self) -> bool {
        let Ok(entries) = std::fs::read_dir(self.0) else {
            return false;
        };
        entries.flatten().any(|entry| {
            let path = entry.path();
            if path.is_dir() {
                return Seat(&path).holds();
            }
            path.extension().is_some_and(|ext| ext == "rs") && marked(&path)
        })
    }

    fn gather(&self, found: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(self.0) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                Seat(&path).gather(found);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                found.push(path);
            }
        }
    }
}

fn marked(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    text.lines()
        .any(|line| line.contains(DERIVE) && line.contains(CASCADE))
}

fn extract(text: &str) -> Vec<Option<String>> {
    let mut found = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut index = 0;
    while index < lines.len() {
        let trimmed = lines[index].trim_start();
        if let Some(after) = trimmed.strip_prefix("#[") {
            let mut held = after.to_string();
            while !held.contains(']') && index + 1 < lines.len() {
                index += 1;
                held.push(' ');
                held.push_str(lines[index].trim());
            }
            if let Some((body, _)) = held.split_once(']')
                && let Some(value) = attribute(body.trim())
            {
                found.push(value);
            }
        }
        index += 1;
    }
    found
}

fn attribute(body: &str) -> Option<Option<String>> {
    let tail = body
        .strip_prefix("plumb::")
        .or_else(|| body.strip_prefix("crate::"))?
        .strip_prefix(PREFIX)?;
    let tail = tail.trim();
    if tail.is_empty() {
        return Some(None);
    }
    let inner = tail.strip_prefix('(')?.strip_suffix(')')?.trim();
    let literal = inner
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .filter(|rest| !rest.contains('"'))
        .unwrap_or(inner);
    Some(Some(literal.to_string()))
}
