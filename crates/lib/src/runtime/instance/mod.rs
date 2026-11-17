mod parse;
mod path;
mod read;

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct Instance {
    pub schema: &'static str,
    pub id: String,
    pub source: Source,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct Source {
    pub kind: &'static str,
    pub path: PathBuf,
}

pub fn current() -> Result<Instance, String> {
    read(&path::current()?)
}

pub fn read(path: &Path) -> Result<Instance, String> {
    Ok(Instance {
        schema: "plumb.instance/v1",
        id: parse(&read::public(path)?)?,
        source: Source {
            kind: "ssh-host-key",
            path: path.to_path_buf(),
        },
    })
}

pub fn parse(public: &[u8]) -> Result<String, String> {
    parse::identity(public)
}
