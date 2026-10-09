use super::super::{Action, Artifact, Build, Source};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "lowercase", deny_unknown_fields)]
pub enum Intent {
    Inspect {},
    Build { source: Source, build: Build },
    Publish { artifact: Box<Artifact> },
    Install { artifact: Box<Artifact> },
    Uninstall {},
    Deploy { artifact: Box<Artifact> },
    Dispose {},
}

impl Intent {
    pub fn action(&self) -> Action {
        match self {
            Self::Inspect {} => Action::Inspect,
            Self::Build { .. } => Action::Build,
            Self::Publish { .. } => Action::Publish,
            Self::Install { .. } => Action::Install,
            Self::Uninstall {} => Action::Uninstall,
            Self::Deploy { .. } => Action::Deploy,
            Self::Dispose {} => Action::Dispose,
        }
    }

    pub fn mutates(&self) -> bool {
        !matches!(self, Self::Inspect {} | Self::Build { .. })
    }
}
