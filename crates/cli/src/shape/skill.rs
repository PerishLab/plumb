use std::path::{Path, PathBuf};

const CODE: [&str; 5] = ["css", "rs", "scss", "ts", "tsx"];

#[derive(Clone)]
pub struct Skill {
    pub name: String,
    pub source: usize,
    pub text: usize,
    pub budget: usize,
    pub files: usize,
}

pub fn read(root: &Path) -> Vec<Skill> {
    let source = roots(root).iter().map(|path| lines(path, &CODE)).sum();
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(root.join("skills")) else {
        return found;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_dir() || kind.is_symlink() {
            continue;
        }
        let (text, files) = prose(&entry.path());
        found.push(Skill {
            name: entry.file_name().to_string_lossy().to_string(),
            source,
            text,
            budget: budget(source),
            files,
        });
    }
    found.sort_by(|left, right| left.name.cmp(&right.name));
    found
}

pub fn budget(source: usize) -> usize {
    let root = (source as f64).sqrt().ceil() as usize;
    (root * 2).clamp(120, 400)
}

fn roots(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    seats(root, &mut found, &["src", "lib", "app/src"]);
    for group in ["apps", "crates", "packages"] {
        let Ok(entries) = std::fs::read_dir(root.join(group)) else {
            continue;
        };
        for entry in entries.flatten() {
            seats(&entry.path(), &mut found, &["src", "lib"]);
        }
    }
    found
}

fn seats(root: &Path, found: &mut Vec<PathBuf>, names: &[&str]) {
    for name in names {
        let path = root.join(name);
        if path.is_dir() {
            found.push(path);
        }
    }
}

fn lines(path: &Path, suffixes: &[&str]) -> usize {
    let Ok(kind) = std::fs::symlink_metadata(path).map(|meta| meta.file_type()) else {
        return 0;
    };
    if kind.is_symlink() {
        return 0;
    }
    if kind.is_file() {
        let held = path.extension().and_then(|value| value.to_str());
        if !held.is_some_and(|suffix| suffixes.contains(&suffix)) {
            return 0;
        }
        return std::fs::read_to_string(path)
            .map(|text| text.lines().count())
            .unwrap_or(0);
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| lines(&entry.path(), suffixes))
        .sum()
}

fn prose(path: &Path) -> (usize, usize) {
    let Ok(kind) = std::fs::symlink_metadata(path).map(|meta| meta.file_type()) else {
        return (0, 0);
    };
    if kind.is_symlink() {
        return (0, 0);
    }
    if kind.is_file() {
        if path.extension().and_then(|value| value.to_str()) != Some("md") {
            return (0, 0);
        }
        let count = std::fs::read_to_string(path)
            .map(|text| text.lines().count())
            .unwrap_or(0);
        return (count, 1);
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return (0, 0);
    };
    entries
        .flatten()
        .map(|entry| prose(&entry.path()))
        .fold((0, 0), |held, found| (held.0 + found.0, held.1 + found.1))
}
