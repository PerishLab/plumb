use crate::shape::depot::{FORMAT, Object};
use serde::{Deserialize, Serialize};

pub const FLOOR: &str = "v0.0.0";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Notes {
    pub format: u32,
    #[serde(default = "floor")]
    pub floor: String,
    pub version: String,
    pub commit: String,
    #[serde(default, rename = "object")]
    pub objects: Vec<Object>,
}

fn floor() -> String {
    FLOOR.to_string()
}

impl Notes {
    pub fn parse(text: &str) -> Result<Self, String> {
        let held: Self =
            toml::from_str(text).map_err(|error| format!("cannot parse release notes: {error}"))?;
        if held.format != FORMAT {
            return Err(format!(
                "release notes format must be {FORMAT}, got {}",
                held.format
            ));
        }
        held.supported()?;
        Ok(held)
    }

    fn supported(&self) -> Result<(), String> {
        let running = plumb::version!("PLUMB");
        let (Ok(held), Ok(least)) = (
            semver::Version::parse(running.trim_start_matches('v')),
            semver::Version::parse(self.floor.trim_start_matches('v')),
        ) else {
            return Err(format!(
                "cannot compare a running {running} with a note floor of {}",
                self.floor
            ));
        };
        if plumb::depot::supports(&held, &least) {
            return Ok(());
        }
        Err(format!(
            "release notes for {} declare a floor of {}, above the running {running}",
            self.version, self.floor
        ))
    }
}

pub fn changelog(version: &str) -> String {
    format!("changelog/{version}")
}
