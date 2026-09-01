use crate::shape::depot::POINTER;
use std::path::{Path, PathBuf};
use std::process::Command;

mod exact;

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

pub use exact::Query;

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
