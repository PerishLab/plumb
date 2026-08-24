use super::notes::{Notes, changelog};
use crate::shape::depot::{LEAF, Manifest, POINTER, Pointer, latest, versions};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const KEY: &str = "plumb";
pub const HOME: &str = "depot";

pub enum Held {
    Factory,
    Seat(PathBuf),
    Blind(String),
}

pub fn root(over: &Path) -> Result<PathBuf, String> {
    if !over.as_os_str().is_empty() {
        return Ok(over.to_path_buf());
    }
    plumb::seat::global(KEY)
        .map(|base| base.join(HOME))
        .ok_or_else(|| "cannot anchor the plumb seat: no home is declared".to_string())
}

pub fn held(over: &Path) -> Held {
    let base = match root(over) {
        Ok(base) => base,
        Err(error) => return Held::Blind(error),
    };
    let marker = base.join(POINTER);
    if !marker.is_file() {
        return Held::Factory;
    }
    let text = match std::fs::read_to_string(&marker) {
        Ok(text) => text,
        Err(error) => return Held::Blind(format!("cannot read {}: {error}", marker.display())),
    };
    match Pointer::parse(&text) {
        Ok(pointer) => Held::Seat(base.join(&pointer.version)),
        Err(error) => Held::Blind(error),
    }
}

impl Held {
    pub fn read(&self, path: &str, factory: &'static str) -> Result<String, String> {
        match self {
            Self::Factory => Ok(factory.to_string()),
            Self::Blind(error) => Err(format!("the plumb depot seat is unreadable: {error}")),
            Self::Seat(base) => {
                let file = base.join(path);
                std::fs::read_to_string(&file)
                    .map_err(|error| format!("cannot read {}: {error}", file.display()))
            }
        }
    }

    pub fn mark(&self) -> Option<String> {
        match self {
            Self::Seat(base) => base
                .file_name()
                .map(|name| name.to_string_lossy().to_string()),
            _ => None,
        }
    }
}

pub fn sync(source: &str, channel: &str, over: &Path) -> Result<String, String> {
    let base = root(over)?;
    let source = source.trim_end_matches('/');
    let text = pull(&format!("{source}/{}", latest(channel)))?
        .ok_or_else(|| format!("depot channel {channel} has no pointer at {source}"))?;
    let pointer = Pointer::parse(&text)?;
    let seat = base.join(&pointer.version);
    let deed = format!("{source}/{}", versions(channel, &pointer.version));
    let raw = pull(&format!("{deed}/{LEAF}"))?
        .ok_or_else(|| format!("depot version {} has no manifest", pointer.version))?;
    let manifest = Manifest::parse(&raw)?;
    for object in &manifest.objects {
        let body = pull(&format!("{deed}/{}", object.path))?
            .ok_or_else(|| format!("depot object {} is absent", object.path))?;
        manifest.verify(&object.path, body.as_bytes())?;
        write(&seat.join(&object.path), &body)?;
    }
    write(&seat.join(LEAF), &raw)?;
    write(&base.join(POINTER), &text)?;
    Ok(format!(
        "synced depot {channel} {} into {}",
        pointer.version,
        base.display()
    ))
}

pub fn notes(source: &str, version: &str) -> Result<Option<Notes>, String> {
    let url = format!(
        "{}/{}/{LEAF}",
        source.trim_end_matches('/'),
        changelog(version)
    );
    match pull(&url)? {
        Some(text) => Notes::parse(&text).map(Some),
        None => Ok(None),
    }
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent", path.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("cannot make {}: {error}", parent.display()))?;
    std::fs::write(path, text).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn pull(url: &str) -> Result<Option<String>, String> {
    let body = tempfile::NamedTempFile::new()
        .map_err(|error| format!("cannot stage a depot fetch: {error}"))?;
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--location",
            "--connect-timeout",
            "5",
            "--max-time",
            "15",
            "--retry",
            "1",
            "--retry-all-errors",
            "--write-out",
            "%{http_code}",
            "--output",
        ])
        .arg(body.path())
        .arg(url)
        .output()
        .map_err(|error| format!("cannot run curl: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot fetch {url}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let code = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if code == "404" {
        return Ok(None);
    }
    if code != "200" {
        return Err(format!("cannot fetch {url}: HTTP {code}"));
    }
    std::fs::read_to_string(body.path())
        .map(Some)
        .map_err(|error| format!("cannot read the depot fetch: {error}"))
}
