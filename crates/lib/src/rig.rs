use crate::config::Cascade;

const RELEASES: &str = "https://releases.plumb.perish.uk";

#[derive(Debug, PartialEq, Cascade)]
pub struct Rig {
    pub home: String,
    pub releases: String,
}

impl Default for Rig {
    fn default() -> Self {
        Rig {
            home: crate::config::data("plumb")
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            releases: RELEASES.to_string(),
        }
    }
}
