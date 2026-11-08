use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Native {
    pub run: u64,
    pub attempt: u64,
    pub commit: String,
    pub job: u64,
}

impl Native {
    pub fn validate(&self) -> Result<(), crate::windtun::qualification::Fault> {
        use super::wire::Wire;
        Wire::require(
            self.run > 0 && self.attempt > 0 && self.job > 0,
            "semantic source requires an exact native attempt and job",
        )?;
        Wire::hex(&self.commit, 40)
    }
}
