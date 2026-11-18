mod declare;
mod parse;
mod path;
mod read;

use std::path::{Path, PathBuf};

pub use declare::{Arguments, Declared, DeclaredPartial, Inputs};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct Instance {
    pub schema: &'static str,
    pub id: String,
    pub source: Source,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct Source {
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}
pub fn current() -> Result<Instance, String> {
    select(None)
}
pub fn select(argument: Option<&str>) -> Result<Instance, String> {
    let hostkey = path::current()?;
    let file = declare::file();
    resolve(&Inputs {
        argument,
        file: file.as_deref(),
        get: &crate::config::value,
        hostkey: &hostkey,
    })
}
pub fn resolve(inputs: &Inputs<'_>) -> Result<Instance, String> {
    match declare::layer(inputs)? {
        Some((source, id)) => Ok(Instance {
            schema: SCHEMA,
            id,
            source,
        }),
        None => read(inputs.hostkey),
    }
}

pub fn read(path: &Path) -> Result<Instance, String> {
    Ok(Instance {
        schema: SCHEMA,
        id: parse(&read::public(path)?)?,
        source: Source {
            kind: "ssh-host-key",
            path: Some(path.to_path_buf()),
        },
    })
}

pub fn parse(public: &[u8]) -> Result<String, String> {
    parse::identity(public)
}

const SCHEMA: &str = "plumb.instance/v1";
