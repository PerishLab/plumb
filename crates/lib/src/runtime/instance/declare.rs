use super::Source;
use radicula_config::{Cascade, Error};
use std::path::{Path, PathBuf};

const LIMIT: usize = 512;

#[derive(Debug, Default, PartialEq, Cascade)]
#[crate::config::prefix]
pub struct Declared {
    #[cascade(name = "instance_id")]
    pub instance: Option<String>,
}

#[derive(clap::Args, Clone, Debug, Default)]
pub struct Arguments {
    #[arg(
        long = "instance-id",
        help = "Declare the instance identifier, above the environment and plumb.toml"
    )]
    pub instance: Option<String>,
}
pub struct Inputs<'a> {
    pub argument: Option<&'a str>,
    pub file: Option<&'a Path>,
    pub get: &'a dyn Fn(&str) -> Option<String>,
    pub hostkey: &'a Path,
}

pub(super) fn file() -> Option<PathBuf> {
    let home = crate::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| crate::config::data("plumb"))?;
    Some(home.join("plumb.toml"))
}

pub(super) fn layer(inputs: &Inputs<'_>) -> Result<Option<(Source, String)>, String> {
    let prefix = Declared::prefix();
    let configured = configured(inputs.file)?;
    let environment = Declared::lookup(&prefix, inputs.get)
        .map_err(|error| error.to_string())?
        .instance
        .flatten();
    if let Some(value) = inputs.argument {
        let id = admit(value, "--instance-id")?;
        return Ok(Some((source("argument", None), id)));
    }
    if let Some(value) = environment {
        let id = admit(&value, &format!("{prefix}_INSTANCE_ID"))?;
        return Ok(Some((source("environment", None), id)));
    }
    if let Some((path, value)) = configured {
        let id = admit(&value, &format!("{} instance_id", path.display()))?;
        return Ok(Some((source("configuration", Some(path)), id)));
    }
    Ok(None)
}

fn configured(file: Option<&Path>) -> Result<Option<(PathBuf, String)>, String> {
    let Some(path) = file else {
        return Ok(None);
    };
    match radicula_config::load::<DeclaredPartial>(path) {
        Ok(partial) => Ok(partial
            .instance
            .flatten()
            .map(|value| (path.to_path_buf(), value))),
        Err(Error::Read { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            Ok(None)
        }
        Err(error) => Err(error.to_string()),
    }
}

fn source(kind: &'static str, path: Option<PathBuf>) -> Source {
    Source { kind, path }
}

fn admit(value: &str, layer: &str) -> Result<String, String> {
    let refusal = |rule: &str| Err(format!("{layer}: instance id {rule}"));
    if value.is_empty() {
        return refusal("must not be empty");
    }
    if value.trim() != value {
        return refusal("must not have leading or trailing whitespace");
    }
    if value.chars().any(char::is_control) {
        return refusal("must not contain control characters");
    }
    if value.len() > LIMIT {
        return refusal(&format!("must be at most {LIMIT} bytes"));
    }
    Ok(value.to_string())
}
