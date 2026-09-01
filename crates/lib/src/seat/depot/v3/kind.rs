use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Configuration,
    Changelog,
    Skill,
    Worker,
    Manager,
}

impl Kind {
    pub fn directory(self) -> &'static str {
        match self {
            Self::Configuration => "configurations",
            Self::Changelog => "changelogs",
            Self::Skill => "skills",
            Self::Worker => "workers",
            Self::Manager => "managers",
        }
    }
}
