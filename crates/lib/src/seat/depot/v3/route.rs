use super::value::Value;
use super::{Kind, LEAF, POINTER};

#[derive(Clone, Copy, Eq, PartialEq)]
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
        super::value::channel(self.channel, self.version)
    }
}

pub fn channel(channel: &str, kind: Kind) -> Result<String, String> {
    Value(channel).channel()?;
    Ok(format!("channels/{channel}/{}/{POINTER}", kind.directory()))
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
