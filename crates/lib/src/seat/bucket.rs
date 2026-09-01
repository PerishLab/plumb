mod s3;

pub use s3::{Condition, Control, Object, Outcome, Policy};
use std::time::Duration;

pub fn fetch(url: &str) -> Result<Option<Vec<u8>>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build()
        .into();
    match agent.get(url).call() {
        Ok(response) => response
            .into_body()
            .read_to_vec()
            .map(Some)
            .map_err(|error| format!("cannot read public bucket object: {error}")),
        Err(ureq::Error::StatusCode(404)) => Ok(None),
        Err(error) => Err(format!("cannot fetch public bucket object: {error}")),
    }
}
