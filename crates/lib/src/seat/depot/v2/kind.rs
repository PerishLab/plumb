use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Configuration,
    Changelog,
    Skill,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::Changelog => "changelog",
            Self::Skill => "skill",
        }
    }
}
