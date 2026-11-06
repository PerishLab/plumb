use serde::{Deserialize, Serialize};

use super::{Snapshot, wire::Wire};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Same,
    Changed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub before: String,
    pub after: String,
    pub anchor: String,
    pub decision: Decision,
    pub reference: String,
}

impl Review {
    pub(super) fn bind(
        &self,
        before: &Snapshot,
        after: &Snapshot,
        anchor: &Snapshot,
    ) -> Result<(), String> {
        Wire::reference(&self.reference)?;
        if self.before != before.digest()?
            || self.after != after.digest()?
            || self.anchor != anchor.digest()?
        {
            return Err("review does not bind the exact before, after and anchor inputs".into());
        }
        if self.before == self.after && self.decision == Decision::Changed {
            return Err("identical boundary inputs cannot claim a changed boundary".into());
        }
        Ok(())
    }
}
