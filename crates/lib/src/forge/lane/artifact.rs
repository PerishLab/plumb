use super::{Build, Digest, Source};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    digest: Digest,
    source: Source,
    build: Option<Build>,
}

impl Artifact {
    pub fn new(digest: Digest, source: Source, build: Option<Build>) -> Self {
        Self {
            digest,
            source,
            build,
        }
    }

    pub fn digest(&self) -> &Digest {
        &self.digest
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    pub fn build(&self) -> Option<&Build> {
        self.build.as_ref()
    }
}
