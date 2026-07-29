use crate::config::Cascade;
use std::path::PathBuf;

const RELEASES: &str = "https://releases.plumb.perish.uk";

#[derive(Debug, PartialEq, Cascade)]
pub struct Rig {
    pub home: String,
    pub releases: String,
    #[cascade(section)]
    pub release: Release,
    #[cascade(section)]
    pub publish: Authority,
    #[cascade(section)]
    pub activate: Authority,
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Release {
    pub root: PathBuf,
    pub channel: String,
    pub version: String,
    pub commit: String,
    pub target: String,
    pub artifacts: PathBuf,
    pub output: PathBuf,
    pub promotion: Option<PathBuf>,
    pub promotion_channel: String,
    pub promotion_version: String,
    pub capsule: PathBuf,
    pub url: String,
    pub activated: bool,
    pub registry_token: String,
}

#[derive(Debug, Default, PartialEq, Cascade)]
#[cascade(section)]
pub struct Authority {
    pub access: String,
    pub secret: String,
    pub bucket: String,
    pub endpoint: String,
}

impl Default for Rig {
    fn default() -> Self {
        Rig {
            home: crate::config::data("plumb")
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            releases: RELEASES.to_string(),
            release: Release {
                root: PathBuf::from("."),
                ..Release::default()
            },
            publish: Authority::default(),
            activate: Authority::default(),
        }
    }
}
