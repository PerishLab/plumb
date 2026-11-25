mod s3;

use crate::runtime::exchange;
pub use s3::{Condition, Control, Object, Outcome, Policy};
use std::time::{Duration, Instant};

const ATTEMPTS: u32 = 3;

enum Fault {
    Transient(Option<u16>, String),
    Final(Option<u16>, String),
}

pub fn fetch(url: &str) -> Result<Option<Vec<u8>>, String> {
    let started = Instant::now();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .max_idle_connections(0)
        .build()
        .into();
    for attempt in 1..=ATTEMPTS {
        let count = attempt as u8;
        match once(&agent, url) {
            Ok(Some(bytes)) => {
                exchange::attempt("bucket.fetch", "ok", Some(200), (count, started.elapsed()));
                return Ok(Some(bytes));
            }
            Ok(None) => {
                exchange::attempt(
                    "bucket.fetch",
                    "missing",
                    Some(404),
                    (count, started.elapsed()),
                );
                return Ok(None);
            }
            Err(Fault::Final(code, error)) => {
                exchange::attempt("bucket.fetch", "error", code, (count, started.elapsed()));
                return Err(error);
            }
            Err(Fault::Transient(code, error)) if attempt == ATTEMPTS => {
                exchange::attempt("bucket.fetch", "error", code, (count, started.elapsed()));
                return Err(error);
            }
            Err(Fault::Transient(_, _)) => {
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
                Fault::Transient(None, format!("cannot read public bucket object: {error}"))
            }),
        Err(ureq::Error::StatusCode(404)) => Ok(None),
        Err(error) => {
            let code = match &error {
                ureq::Error::StatusCode(code) => Some(*code),
                _ => None,
            };
            let message = format!("cannot fetch public bucket object: {error}");
            if transient(&error) {
                Err(Fault::Transient(code, message))
            } else {
                Err(Fault::Final(code, message))
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
