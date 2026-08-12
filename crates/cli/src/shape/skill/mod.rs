use std::io::ErrorKind;
use std::path::Path;

mod config;
mod source;

pub use config::{Config, Strategy};

#[derive(Clone)]
pub struct Skill {
    pub name: String,
    pub source: usize,
    pub text: usize,
    pub budget: Option<usize>,
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

pub struct Read {
    pub config: Config,
    pub held: Vec<Skill>,
    pub unread: Option<String>,
    pub present: bool,
}

impl Read {
    pub fn empty(root: &Path) -> Self {
        Self {
            config: config::read(root),
            held: Vec::new(),
            unread: None,
            present: root.join("skills").is_dir(),
        }
    }
}

pub fn read(root: &Path) -> Read {
    let config = config::read(root);
    let mut found = Read {
        config,
        held: Vec::new(),
        unread: None,
        present: false,
    };
    let entries = match std::fs::read_dir(root.join("skills")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return found,
        Err(error) => {
            found.unread = Some(error.to_string());
            return found;
        }
    };
    found.present = true;
    let source = source::read(root);
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
            budget: found
                .config
                .strategy()
                .map(|held| held.budget(source.lines)),
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
