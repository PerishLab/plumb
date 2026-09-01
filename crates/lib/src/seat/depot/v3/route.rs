use super::value::Value;
use super::{Kind, LEAF, POINTER};

#[derive(Clone, Copy)]
pub struct Route<'a> {
    pub channel: &'a str,
    pub kind: Kind,
    pub version: &'a str,
}

impl<'a> Route<'a> {
    pub fn new(channel: &'a str, kind: Kind, version: &'a str) -> Self {
        Self {
            channel,
            kind,
            version,
        }
    }

    fn validate(self) -> Result<(), String> {
        Value(self.channel).component("release channel")?;
        super::value::version(self.version)
    }
}

pub fn generation(route: Route<'_>, generation: &str) -> Result<String, String> {
    route.validate()?;
    Value(generation).digest("depot generation")?;
    Ok(format!(
        "channels/{}/{}/versions/{}/generations/{generation}",
        route.channel,
        route.kind.directory(),
        route.version
    ))
}

pub fn manifest(source: &str, route: Route<'_>, held: &str) -> Result<String, String> {
    Value(source).source()?;
    generation(route, held).map(|route| format!("{}/{route}/{LEAF}", source.trim_end_matches('/')))
}

pub fn latest(route: Route<'_>) -> Result<String, String> {
    route.validate()?;
    Ok(format!(
        "channels/{}/{}/versions/{}/{POINTER}",
        route.channel,
        route.kind.directory(),
        route.version
    ))
}
