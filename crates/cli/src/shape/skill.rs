use std::io::ErrorKind;
use std::path::Path;

const CODE: [&str; 5] = ["css", "rs", "scss", "ts", "tsx"];
pub const DOCUMENTS: [&str; 3] = ["PATHS.md", "SCENARIOS.md", "SKILL.md"];

#[derive(Clone)]
pub struct Skill {
    pub name: String,
    pub source: usize,
    pub text: usize,
    pub budget: usize,
    pub files: usize,
    pub entries: Vec<String>,
    pub regular: Vec<String>,
    pub unread: Unread,
    pub seat: bool,
}

#[derive(Clone, Default)]
pub struct Unread {
    pub form: Vec<String>,
    pub text: Vec<String>,
    pub source: Vec<String>,
}

#[derive(Default)]
pub struct Read {
    pub held: Vec<Skill>,
    pub unread: Option<String>,
}

#[derive(Default)]
struct Metric {
    lines: usize,
    unread: Vec<String>,
}

struct Source<'a>(&'a Path);

pub fn read(root: &Path) -> Read {
    let source = Source(root).read();
    let mut found = Read::default();
    let entries = match std::fs::read_dir(root.join("skills")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return found,
        Err(error) => {
            found.unread = Some(error.to_string());
            return found;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                found.unread = Some(error.to_string());
                continue;
            }
        };
        let name = match entry.file_name().into_string() {
            Ok(name) => name,
            Err(_) => {
                found.unread = Some("skills contains a non-UTF-8 entry".to_string());
                continue;
            }
        };
        let mut skill = Skill {
            name,
            source: source.lines,
            text: 0,
            budget: budget(source.lines),
            files: 0,
            entries: Vec::new(),
            regular: Vec::new(),
            unread: Unread {
                source: source.unread.clone(),
                ..Unread::default()
            },
            seat: false,
        };
        inspect(&entry.path(), &mut skill);
        found.held.push(skill);
    }
    found.held.sort_by(|left, right| left.name.cmp(&right.name));
    found
}

pub fn budget(source: usize) -> usize {
    let root = (source as f64).sqrt().ceil() as usize;
    (root * 2).clamp(120, 400)
}

fn inspect(path: &Path, skill: &mut Skill) {
    let kind = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta.file_type(),
        Err(error) => {
            skill.unread.form.push(error.to_string());
            skill.unread.text.push(error.to_string());
            return;
        }
    };
    if kind.is_symlink() || !kind.is_dir() {
        return;
    }
    skill.seat = true;
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) => {
            skill.unread.form.push(error.to_string());
            skill.unread.text.push(error.to_string());
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                let error = error.to_string();
                skill.unread.form.push(error.clone());
                skill.unread.text.push(error);
                continue;
            }
        };
        let name = match entry.file_name().into_string() {
            Ok(name) => name,
            Err(_) => {
                let error = "skill contains a non-UTF-8 entry".to_string();
                skill.unread.form.push(error.clone());
                skill.unread.text.push(error);
                continue;
            }
        };
        skill.entries.push(name.clone());
        match std::fs::symlink_metadata(entry.path()) {
            Ok(meta) if meta.file_type().is_file() => skill.regular.push(name),
            Ok(_) => {}
            Err(error) => skill.unread.form.push(error.to_string()),
        }
    }
    skill.entries.sort();
    skill.regular.sort();
    prose(path, path, skill);
}

fn prose(root: &Path, path: &Path, skill: &mut Skill) {
    let kind = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta.file_type(),
        Err(error) => {
            skill.unread.text.push(error.to_string());
            return;
        }
    };
    if kind.is_symlink() {
        return;
    }
    if kind.is_file() {
        if path.extension().and_then(|value| value.to_str()) != Some("md") {
            return;
        }
        skill.files += 1;
        let name = path.strip_prefix(root).unwrap_or(path).display();
        match std::fs::read(path) {
            Ok(bytes) => match std::str::from_utf8(&bytes) {
                Ok(text) => skill.text += text.lines().count(),
                Err(_) => skill.unread.text.push(format!("{name} is not UTF-8")),
            },
            Err(error) => skill
                .unread
                .text
                .push(format!("cannot read {name}: {error}")),
        }
        return;
    }
    if !kind.is_dir() {
        return;
    }
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) => {
            skill.unread.text.push(error.to_string());
            return;
        }
    };
    for entry in entries {
        match entry {
            Ok(entry) => prose(root, &entry.path(), skill),
            Err(error) => skill.unread.text.push(error.to_string()),
        }
    }
}

impl Source<'_> {
    fn read(&self) -> Metric {
        let mut found = Metric::default();
        self.seats(self.0, &mut found, &["src", "lib", "app/src"]);
        for group in ["apps", "crates", "packages"] {
            let entries = match std::fs::read_dir(self.0.join(group)) {
                Ok(entries) => entries,
                Err(error) if error.kind() == ErrorKind::NotFound => continue,
                Err(error) => {
                    found.unread.push(error.to_string());
                    continue;
                }
            };
            for entry in entries {
                match entry {
                    Ok(entry) => self.seats(&entry.path(), &mut found, &["src", "lib"]),
                    Err(error) => found.unread.push(error.to_string()),
                }
            }
        }
        found
    }

    fn seats(&self, root: &Path, found: &mut Metric, names: &[&str]) {
        for name in names {
            let path = root.join(name);
            match std::fs::symlink_metadata(&path) {
                Ok(meta) if meta.file_type().is_dir() => self.lines(&path, found),
                Ok(meta) if meta.file_type().is_symlink() => found
                    .unread
                    .push(format!("source seat {} is a symlink", path.display())),
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => found.unread.push(error.to_string()),
            }
        }
    }

    fn lines(&self, path: &Path, found: &mut Metric) {
        let kind = match std::fs::symlink_metadata(path) {
            Ok(meta) => meta.file_type(),
            Err(error) => {
                found.unread.push(error.to_string());
                return;
            }
        };
        if kind.is_symlink() {
            found
                .unread
                .push(format!("source path {} is a symlink", path.display()));
            return;
        }
        if kind.is_file() {
            let held = path.extension().and_then(|value| value.to_str());
            if !held.is_some_and(|suffix| CODE.contains(&suffix)) {
                return;
            }
            match std::fs::read(path) {
                Ok(bytes) => found.lines += count(&bytes),
                Err(error) => found.unread.push(error.to_string()),
            }
            return;
        }
        if !kind.is_dir() {
            let held = path.extension().and_then(|value| value.to_str());
            if held.is_some_and(|suffix| CODE.contains(&suffix)) {
                found.unread.push(format!(
                    "source path {} is not a regular file",
                    path.display()
                ));
            }
            return;
        }
        let entries = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(error) => {
                found.unread.push(error.to_string());
                return;
            }
        };
        for entry in entries {
            match entry {
                Ok(entry) => self.lines(&entry.path(), found),
                Err(error) => found.unread.push(error.to_string()),
            }
        }
    }
}

fn count(bytes: &[u8]) -> usize {
    let lines = bytes.iter().filter(|byte| **byte == b'\n').count();
    lines + usize::from(!bytes.is_empty() && !bytes.ends_with(b"\n"))
}
