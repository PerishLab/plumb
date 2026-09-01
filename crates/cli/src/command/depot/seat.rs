use super::notes::{Notes, changelog};
use crate::shape::depot::{LEAF, POINTER};
use std::path::Path;
use std::process::Command;

mod exact;

pub enum Held {
    Absent,
    Seat(Box<plumb::depot::Rules>),
    Blind(String),
}

pub use plumb::depot::root;

pub fn held(over: &Path) -> Held {
    if let Some(path) = plumb::config::value("PLUMB_GUARD_CONFIGURATION") {
        return plumb::depot::Rules::guard(Path::new(&path), plumb::version!("PLUMB"))
            .map(|seat| Held::Seat(Box::new(seat)))
            .unwrap_or_else(Held::Blind);
    }
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

pub use exact::Query;

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

pub use exact::read as exact;

pub(super) fn pull(url: &str) -> Result<Option<String>, String> {
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
