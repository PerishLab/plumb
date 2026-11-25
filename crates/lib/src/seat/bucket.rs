mod s3;

pub use s3::{Condition, Control, Object, Outcome, Policy};
use std::time::Duration;

const ATTEMPTS: u32 = 3;

enum Fault {
    Transient(String),
    Final(String),
}

pub fn fetch(url: &str) -> Result<Option<Vec<u8>>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .max_idle_connections(0)
        .build()
        .into();
    for attempt in 1..=ATTEMPTS {
        match once(&agent, url) {
            Ok(found) => return Ok(found),
            Err(Fault::Final(error)) => return Err(error),
            Err(Fault::Transient(error)) if attempt == ATTEMPTS => return Err(error),
            Err(Fault::Transient(_)) => {
                std::thread::sleep(Duration::from_millis(200 * u64::from(attempt)));
            }
        }
    }
    unreachable!()
}

fn once(agent: &ureq::Agent, url: &str) -> Result<Option<Vec<u8>>, Fault> {
    match agent.get(url).call() {
        Ok(response) => response
            .into_body()
            .read_to_vec()
            .map(Some)
            .map_err(|error| {
                Fault::Transient(format!("cannot read public bucket object: {error}"))
            }),
        Err(ureq::Error::StatusCode(404)) => Ok(None),
        Err(error) => {
            let message = format!("cannot fetch public bucket object: {error}");
            if transient(&error) {
                Err(Fault::Transient(message))
            } else {
                Err(Fault::Final(message))
            }
        }
    }
}

fn transient(error: &ureq::Error) -> bool {
    matches!(
        error,
        ureq::Error::Io(_)
            | ureq::Error::ConnectionFailed
            | ureq::Error::BodyStalled
            | ureq::Error::Protocol(_)
            | ureq::Error::Timeout(ureq::Timeout::Connect)
    )
}

#[cfg(test)]
mod tests;
