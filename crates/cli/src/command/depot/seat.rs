use super::notes::{Notes, changelog};
use crate::shape::depot::{LEAF, POINTER, Pointer, latest, versions};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const KEY: &str = "plumb";

pub enum Held {
    Absent,
    Seat(Box<plumb::depot::Rules>),
    Blind(String),
}

pub fn root(over: &Path) -> Result<PathBuf, String> {
    plumb::depot::root(over)
}

pub fn held(over: &Path) -> Held {
    let base = match root(over) {
        Ok(base) => base,
        Err(error) => return Held::Blind(error),
    };
    if !base.join(plumb::depot::v2::POINTER).is_file() && !base.join(POINTER).is_file() {
        return Held::Absent;
    }
    match plumb::depot::Rules::at(&base, plumb::version!("PLUMB")) {
        Ok(seat) => Held::Seat(Box::new(seat)),
        Err(error) => Held::Blind(error),
    }
}

impl Held {
    pub fn read(&self, path: &str, factory: &'static str) -> Result<String, String> {
        match self {
            Self::Absent => Ok(factory.to_string()),
            Self::Blind(error) => Err(format!("the plumb depot seat is unreadable: {error}")),
            Self::Seat(seat) => seat.read(path),
        }
    }

    pub fn mark(&self) -> Option<String> {
        match self {
            Self::Seat(seat) => Some(seat.mark().to_string()),
            _ => None,
        }
    }
}

pub fn sync(source: &str, channel: &str, over: &Path) -> Result<String, String> {
    let base = root(over)?;
    let source = source.trim_end_matches('/');
    Remote {
        source,
        channel,
        base: &base,
    }
    .sync()
}

struct Remote<'a> {
    source: &'a str,
    channel: &'a str,
    base: &'a Path,
}

impl Remote<'_> {
    fn sync(&self) -> Result<String, String> {
        if let Some(held) = self.modern()? {
            return Ok(held);
        }
        self.legacy()
    }

    fn modern(&self) -> Result<Option<String>, String> {
        let kind = plumb::depot::v2::Kind::Configuration;
        let key = plumb::depot::v2::latest(KEY, kind, self.channel)?;
        let Some(text) = pull(&format!("{}/{key}", self.source))? else {
            return Ok(None);
        };
        let pointer = plumb::depot::v2::Pointer::parse(&text)?;
        let standing = (
            pointer.source.as_str(),
            pointer.release.product.as_str(),
            pointer.release.channel.as_str(),
            pointer.derivative,
        );
        if standing != (self.source, KEY, self.channel, kind) {
            return Err(format!(
                "depot v2 pointer does not name {KEY} configuration channel {} at {}",
                self.channel, self.source
            ));
        }
        let route = plumb::depot::v2::snapshots(
            &pointer.release,
            pointer.derivative,
            &pointer.snapshot.timestamp,
        )?;
        let deed = format!("{}/{route}", self.source);
        let raw = pull(&format!("{deed}/{}", plumb::depot::v2::LEAF))?.ok_or_else(|| {
            format!(
                "depot snapshot {} has no manifest",
                pointer.snapshot.timestamp
            )
        })?;
        let manifest = plumb::depot::v2::Manifest::parse(&raw)?;
        pointer.bind(&manifest, raw.as_bytes())?;
        let seat =
            plumb::depot::v2::local(self.base, &pointer.release, &pointer.snapshot.timestamp)?;
        for object in &manifest.objects {
            let body = pull(&format!("{deed}/{}", object.path))?
                .ok_or_else(|| format!("depot object {} is absent", object.path))?;
            manifest.verify(&object.path, body.as_bytes())?;
            write(&seat.join(&object.path), &body)?;
        }
        write(&seat.join(plumb::depot::v2::LEAF), &raw)?;
        write(&self.base.join(plumb::depot::v2::POINTER), &text)?;
        Ok(Some(format!(
            "synced depot {} {} into {}",
            self.channel,
            pointer.snapshot.timestamp,
            self.base.display()
        )))
    }

    fn legacy(&self) -> Result<String, String> {
        let text =
            pull(&format!("{}/{}", self.source, latest(self.channel)))?.ok_or_else(|| {
                format!(
                    "depot channel {} has no pointer at {}",
                    self.channel, self.source
                )
            })?;
        let pointer = Pointer::parse(&text)?;
        let seat = self.base.join(&pointer.version);
        let deed = format!(
            "{}/{}",
            self.source,
            versions(self.channel, &pointer.version)
        );
        let raw = pull(&format!("{deed}/{LEAF}"))?
            .ok_or_else(|| format!("depot version {} has no manifest", pointer.version))?;
        let manifest = plumb::depot::Manifest::parse(&raw)?;
        for object in &manifest.objects {
            let body = pull(&format!("{deed}/{}", object.path))?
                .ok_or_else(|| format!("depot object {} is absent", object.path))?;
            manifest.verify(&object.path, body.as_bytes())?;
            write(&seat.join(&object.path), &body)?;
        }
        write(&seat.join(LEAF), &raw)?;
        write(&self.base.join(POINTER), &text)?;
        Ok(format!(
            "synced depot {} {} into {}",
            self.channel,
            pointer.version,
            self.base.display()
        ))
    }
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

pub struct Query<'a> {
    pub source: &'a str,
    pub product: &'a str,
    pub channel: &'a str,
    pub version: &'a str,
    pub derivative: plumb::depot::v2::Kind,
}

pub fn derivative(query: Query<'_>) -> Result<Option<plumb::depot::v2::Manifest>, String> {
    let source = query.source.trim_end_matches('/');
    let key = plumb::depot::v2::latest(query.product, query.derivative, query.channel)?;
    let url = format!("{source}/{key}");
    let Some(text) = pull(&url)? else {
        return Ok(None);
    };
    let pointer = plumb::depot::v2::Pointer::parse(&text)?;
    let standing = (
        pointer.source.as_str(),
        pointer.release.product.as_str(),
        pointer.release.channel.as_str(),
        pointer.release.version.as_str(),
        pointer.derivative,
    );
    let wanted = (
        source,
        query.product,
        query.channel,
        query.version,
        query.derivative,
    );
    if standing != wanted {
        return Ok(None);
    }
    let base = plumb::depot::v2::snapshots(
        &pointer.release,
        pointer.derivative,
        &pointer.snapshot.timestamp,
    )?;
    let route = format!("{}/{}/{}", source, base, plumb::depot::v2::LEAF);
    let body = pull(&route)?.ok_or_else(|| {
        format!(
            "depot pointer {} names an absent manifest",
            pointer.snapshot.timestamp
        )
    })?;
    let manifest = plumb::depot::v2::Manifest::parse(&body)?;
    pointer.bind(&manifest, body.as_bytes())?;
    Ok(Some(manifest))
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
