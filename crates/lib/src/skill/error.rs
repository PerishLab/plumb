use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    Read(PathBuf, String),
    Write(PathBuf, String),
    Parse(String),
    Schema(u32),
    Fetch(String, String),
    Digest(String, String),
    Unpack(String),
    Absent,
    Loose,
    Shape(String),
    Channel(String),
    Floating(String),
    Managed(String),
    Stage,
    Version(String),
    Bare,
    Named(PathBuf),
    Occupied(PathBuf),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(path, why) => write!(f, "cannot read {}: {why}", path.display()),
            Self::Write(path, why) => write!(f, "cannot write {}: {why}", path.display()),
            Self::Parse(why) => write!(f, "malformed json: {why}"),
            Self::Schema(seen) => write!(f, "unknown state schema {seen}"),
            Self::Fetch(url, why) => write!(f, "cannot fetch {url}: {why}"),
            Self::Digest(want, seen) => write!(f, "digest mismatch: want {want} got {seen}"),
            Self::Unpack(why) => write!(f, "cannot unpack: {why}"),
            Self::Absent => write!(f, "selected version carries no skill"),
            Self::Loose => write!(f, "release names no digest for the skill artifact"),
            Self::Shape(name) => write!(f, "unexpected artifact {name}"),
            Self::Channel(channel) => write!(f, "invalid release channel: {channel}"),
            Self::Floating(channel) => {
                write!(f, "non-stable channel {channel} requires an exact version")
            }
            Self::Managed(channel) => {
                write!(f, "managed skills only admit stable, not {channel}")
            }
            Self::Stage => write!(f, "stable belongs in managed skill seats, not staging"),
            Self::Version(version) => write!(f, "invalid release version: {version}"),
            Self::Bare => write!(f, "no agent skill directory was found"),
            Self::Named(path) => write!(f, "path must end with the skill name: {}", path.display()),
            Self::Occupied(path) => {
                write!(f, "staging path must not exist: {}", path.display())
            }
        }
    }
}

impl std::error::Error for Error {}
