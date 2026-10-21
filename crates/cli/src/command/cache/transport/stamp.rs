use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Stamp {
    seconds: u64,
    nanos: u32,
}

impl Stamp {
    pub fn at(time: SystemTime) -> Result<Self, String> {
        let elapsed = time
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            seconds: elapsed.as_secs(),
            nanos: elapsed.subsec_nanos(),
        })
    }

    pub fn now() -> Result<Self, String> {
        Self::at(SystemTime::now())
    }

    pub fn time(self) -> Result<SystemTime, String> {
        if self.nanos >= 1_000_000_000 {
            return Err("invalid Guard material timestamp".into());
        }
        UNIX_EPOCH
            .checked_add(Duration::new(self.seconds, self.nanos))
            .ok_or_else(|| "Guard material timestamp exceeds this platform".into())
    }

    pub fn fresh(self) -> Result<SystemTime, String> {
        let after = self
            .time()?
            .checked_add(Duration::from_millis(1))
            .ok_or("Guard material timestamp cannot advance")?;
        Ok(SystemTime::now().max(after))
    }
}
