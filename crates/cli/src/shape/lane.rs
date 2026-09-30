use std::collections::BTreeSet;
use std::path::Path;

const SEATS: [&str; 2] = [".github/workflows", ".forgejo/workflows"];

#[derive(Default)]
pub struct Evidence {
    actual: BTreeSet<String>,
    containers: Vec<(String, String)>,
}

impl Evidence {
    pub fn names(&self) -> BTreeSet<String> {
        self.actual
            .iter()
            .filter_map(|path| name(path))
            .map(str::to_string)
            .collect()
    }

    pub fn containers(&self) -> Vec<(String, String)> {
        self.containers.clone()
    }
}

pub fn name(path: &str) -> Option<&str> {
    let (seat, file) = path.rsplit_once('/')?;
    if !SEATS.contains(&seat) {
        return None;
    }
    file.strip_suffix(".yml")
        .or_else(|| file.strip_suffix(".yaml"))
}

pub fn read(root: &Path) -> Evidence {
    let mut actual = BTreeSet::new();
    let mut containers = Vec::new();
    for seat in SEATS {
        let Ok(entries) = std::fs::read_dir(root.join(seat)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = format!("{seat}/{}", entry.file_name().to_string_lossy());
            if name(&path).is_none() {
                continue;
            }
            containers.extend(carried(&entry.path(), &path));
            actual.insert(path);
        }
    }
    Evidence { actual, containers }
}

fn carried(file: &Path, path: &str) -> Vec<(String, String)> {
    std::fs::read_to_string(file)
        .map(|text| {
            images(&text)
                .into_iter()
                .map(|image| (path.to_string(), image))
                .collect()
        })
        .unwrap_or_default()
}

fn images(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut block = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - trimmed.len();
        if block.is_some_and(|base| indent > base) {
            carry(trimmed, &mut found, &mut block);
            continue;
        }
        block = None;
        let Some(value) = trimmed.strip_prefix("container:") else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            block = Some(indent);
        } else if let Some(value) = mapping(value).or_else(|| scalar(value)) {
            found.push(value);
        }
    }
    found
}

fn carry(line: &str, found: &mut Vec<String>, block: &mut Option<usize>) {
    let Some(value) = line.strip_prefix("image:").and_then(scalar) else {
        return;
    };
    found.push(value);
    *block = None;
}

fn mapping(value: &str) -> Option<String> {
    let held = value.strip_prefix('{')?.strip_suffix('}')?;
    scalar(held.trim().strip_prefix("image:")?)
}

fn scalar(value: &str) -> Option<String> {
    let value = value
        .trim()
        .split_once(" #")
        .map_or(value.trim(), |(held, _)| held);
    let value = if let Some(held) = value
        .strip_prefix('"')
        .and_then(|held| held.strip_suffix('"'))
    {
        held
    } else if let Some(held) = value
        .strip_prefix('\'')
        .and_then(|held| held.strip_suffix('\''))
    {
        held
    } else {
        value
    };
    (!value.is_empty()).then(|| value.to_string())
}
