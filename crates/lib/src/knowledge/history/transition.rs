use super::super::{Intent, Reference};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub intent: Intent,
    pub history: Reference,
    pub source: Option<Reference>,
}
