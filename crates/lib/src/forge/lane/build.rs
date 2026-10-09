use super::Digest;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    inputs: Digest,
    world: Digest,
}

impl Build {
    pub fn new(inputs: Digest, world: Digest) -> Self {
        Self { inputs, world }
    }

    pub fn inputs(&self) -> &Digest {
        &self.inputs
    }

    pub fn world(&self) -> &Digest {
        &self.world
    }
}
