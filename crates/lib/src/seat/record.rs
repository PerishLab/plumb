use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Error {
    Read(PathBuf, std::io::Error),
    Parse(PathBuf, serde_json::Error),
    Schema(PathBuf, u32),
    Absent(PathBuf),
    Write(PathBuf, std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(path, why) => write!(f, "cannot read {}: {why}", path.display()),
            Self::Parse(path, why) => write!(f, "malformed record {}: {why}", path.display()),
            Self::Schema(path, seen) => {
                write!(f, "unknown record schema {seen} in {}", path.display())
            }
            Self::Absent(path) => write!(f, "record {} names no schema", path.display()),
            Self::Write(path, why) => write!(f, "cannot write {}: {why}", path.display()),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Serialize)]
struct Envelope<'a, T> {
    schema: u32,
    #[serde(flatten)]
    body: &'a T,
}

pub fn read<T: DeserializeOwned>(path: &Path, schema: u32) -> Result<Option<T>, Error> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(Error::Read(path.to_path_buf(), error)),
    };
    decode(path, &text, schema).map(Some)
}

pub fn decode<T: DeserializeOwned>(path: &Path, text: &str, schema: u32) -> Result<T, Error> {
    let parse = |error| Error::Parse(path.to_path_buf(), error);
    let mut value: serde_json::Value = serde_json::from_str(text).map_err(parse)?;
    let seen = value
        .as_object_mut()
        .and_then(|fields| fields.remove("schema"))
        .and_then(|seen| seen.as_u64());
    match seen.map(u32::try_from) {
        Some(Ok(seen)) if seen == schema => serde_json::from_value(value).map_err(parse),
        Some(Ok(seen)) => Err(Error::Schema(path.to_path_buf(), seen)),
        _ => Err(Error::Absent(path.to_path_buf())),
    }
}

pub fn encode<T: Serialize>(schema: u32, body: &T) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = serde_json::to_vec_pretty(&Envelope { schema, body })?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn write<T: Serialize>(path: &Path, schema: u32, body: &T) -> Result<(), Error> {
    let bytes = encode(schema, body)
        .map_err(|error| Error::Write(path.to_path_buf(), std::io::Error::other(error)))?;
    super::replace::write(path, &bytes).map_err(|error| Error::Write(path.to_path_buf(), error))
}
