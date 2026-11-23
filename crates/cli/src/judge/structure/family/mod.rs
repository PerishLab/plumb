use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[cfg(test)]
mod proof;

const FAMILY: &[&str] = &[
    "codehull", "concord", "crest", "design", "dynet", "ectropy", "ensign", "hardrig", "images",
    "ironbed", "keel", "locus", "noema", "plumb", "radicula", "runseal", "santi", "sidecar",
    "stim",
];

const DAY: Duration = Duration::from_secs(60 * 60 * 24);

pub enum Widths {
    Known(BTreeMap<String, usize>),
    Blind(String),
}

pub fn observed() -> Widths {
    if let Some(path) = plumb::config::value("PLUMB_FAMILY_WIDTHS") {
        return loaded(Path::new(&path));
    }
    if let Some(held) = cached() {
        return held;
    }
    match collect() {
        Ok(held) => {
            store(&held);
            Widths::Known(held)
        }
        Err(error) => Widths::Blind(error),
    }
}

fn loaded(path: &Path) -> Widths {
    match read(path) {
        Ok(held) => Widths::Known(held),
        Err(error) => Widths::Blind(error),
    }
}

fn cached() -> Option<Widths> {
    let path = place()?;
    let metadata = std::fs::metadata(&path).ok()?;
    let age = SystemTime::now()
        .duration_since(metadata.modified().ok()?)
        .ok()?;
    (age <= DAY).then(|| loaded(&path))
}

fn place() -> Option<PathBuf> {
    plumb::config::value("PLUMB_HOME")
        .map(|home| PathBuf::from(home).join("state").join("family-widths.json"))
}

fn store(held: &BTreeMap<String, usize>) {
    let Some(path) = place() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(body) = serde_json::to_vec(held) {
        let _ = std::fs::write(path, body);
    }
}

fn read(path: &Path) -> Result<BTreeMap<String, usize>, String> {
    let body = std::fs::read(path)
        .map_err(|error| format!("cannot read family widths at {}: {error}", path.display()))?;
    serde_json::from_slice(&body).map_err(|error| format!("family widths do not parse: {error}"))
}

fn collect() -> Result<BTreeMap<String, usize>, String> {
    let mut found = BTreeMap::new();
    for name in FAMILY {
        merge(&mut found, Product(name).widths()?);
    }
    Ok(found)
}

fn merge(found: &mut BTreeMap<String, usize>, held: BTreeMap<String, usize>) {
    for (name, width) in held {
        found
            .entry(name)
            .and_modify(|was| *was = (*was).max(width))
            .or_insert(width);
    }
}

struct Product<'a>(&'a str);

impl Product<'_> {
    fn widths(&self) -> Result<BTreeMap<String, usize>, String> {
        let Some(marker) = self.marker()? else {
            return Ok(BTreeMap::new());
        };
        let Some(record) = self.record(&marker)? else {
            return Ok(BTreeMap::new());
        };
        counted(&record)
    }

    fn marker(&self) -> Result<Option<String>, String> {
        if let Some(held) = self.channel()? {
            return Ok(Some(held));
        }
        self.tagged()
    }

    fn channel(&self) -> Result<Option<String>, String> {
        let url = format!("https://releases.{}/v1/channels/stable.json", self.0);
        let Some(body) = fetch(&url)? else {
            return Ok(None);
        };
        let held: Value = serde_json::from_slice(&body)
            .map_err(|error| format!("{url} does not parse: {error}"))?;
        Ok(held["releaseVersion"].as_str().map(str::to_string))
    }

    fn tagged(&self) -> Result<Option<String>, String> {
        let url = format!("https://github.com/PerishLab/{}.git", self.0);
        let output = std::process::Command::new("git")
            .args(["ls-remote", "--tags", &url])
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(|error| format!("cannot list tags at {url}: {error}"))?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            let error = error.trim();
            return Err(format!("cannot list tags at {url}: {error}"));
        }
        let listing = String::from_utf8_lossy(&output.stdout);
        Ok(
            plumb::land::rejoin::latest(plumb::land::rejoin::tags(&listing))
                .map(|held| held.marker),
        )
    }

    fn record(&self, marker: &str) -> Result<Option<Record>, String> {
        let url = format!(
            "https://releases.{}/v1/releases/stable/{marker}/distribution.json",
            self.0
        );
        let Some(body) = fetch(&url)? else {
            return Ok(None);
        };
        parsed(&body, marker, &url).map(Some)
    }
}

struct Record {
    state: String,
    repository: String,
    commit: String,
    media: BTreeMap<String, String>,
}

fn parsed(body: &[u8], marker: &str, url: &str) -> Result<Record, String> {
    let held: Value =
        serde_json::from_slice(body).map_err(|error| format!("{url} does not parse: {error}"))?;
    if held["marker"].as_str() != Some(marker) {
        return Err(format!("{url} names {}", held["marker"]));
    }
    let text = |key: &str| held[key].as_str().unwrap_or_default().to_string();
    let media = held["media"]
        .as_object()
        .map(|media| {
            media
                .iter()
                .map(|(name, state)| (name.clone(), state.as_str().unwrap_or_default().to_string()))
                .collect()
        })
        .unwrap_or_default();
    Ok(Record {
        state: text("state"),
        repository: text("repository"),
        commit: text("commit"),
        media,
    })
}

fn counted(record: &Record) -> Result<BTreeMap<String, usize>, String> {
    if record.state != "complete" {
        return Ok(BTreeMap::new());
    }
    let packed: Vec<String> = record
        .media
        .iter()
        .filter(|(name, state)| packaged(name) && *state == "published")
        .map(|(name, _)| name.clone())
        .collect();
    if packed.is_empty() {
        return Ok(BTreeMap::new());
    }
    let Some(held) = manifest(&record.repository, &record.commit)? else {
        return Err(format!(
            "cannot read plumb.toml at {}@{}",
            record.repository, record.commit
        ));
    };
    Ok(packed
        .into_iter()
        .map(|name| {
            let width = held.get(&name).copied().unwrap_or(0);
            (name, width)
        })
        .collect())
}

fn packaged(name: &str) -> bool {
    name == "cargo" || name == "npm"
}

fn manifest(repository: &str, commit: &str) -> Result<Option<BTreeMap<String, usize>>, String> {
    if !trusted(repository) || !hashed(commit) {
        return Err("release record names an unusable repository or commit".into());
    }
    let url = format!("https://raw.githubusercontent.com/{repository}/{commit}/plumb.toml");
    let Some(body) = fetch(&url)? else {
        return Ok(None);
    };
    let text = String::from_utf8(body).map_err(|_| format!("{url} is not utf-8"))?;
    sections(&text)
        .map(Some)
        .map_err(|error| format!("{url} {error}"))
}

fn trusted(repository: &str) -> bool {
    let Some((owner, name)) = repository.split_once('/') else {
        return false;
    };
    token(owner) && token(name) && !name.contains('/')
}

fn token(held: &str) -> bool {
    !held.is_empty() && held.bytes().all(glyph)
}

fn glyph(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b".-_".contains(&byte)
}

fn hashed(commit: &str) -> bool {
    (7..=40).contains(&commit.len()) && commit.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sections(text: &str) -> Result<BTreeMap<String, usize>, String> {
    let held: toml::Value =
        toml::from_str(text).map_err(|error| format!("does not parse: {error}"))?;
    let mut found = BTreeMap::new();
    let Some(release) = held.get("release").and_then(toml::Value::as_table) else {
        return Ok(found);
    };
    for (name, value) in release {
        let Some(packages) = value.get("packages").and_then(toml::Value::as_array) else {
            continue;
        };
        found.insert(name.clone(), packages.len());
    }
    Ok(found)
}

fn fetch(url: &str) -> Result<Option<Vec<u8>>, String> {
    plumb::bucket::fetch(url)
}
