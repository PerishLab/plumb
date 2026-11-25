use super::{ATTEMPTS, Control, Outcome};
use crate::runtime::exchange;
use std::time::{Duration, Instant};

impl Control {
    pub fn head(&self, key: &str) -> Result<Outcome<()>, String> {
        let started = Instant::now();
        for attempt in 0..ATTEMPTS {
            pause(attempt);
            let signed = self.sign("HEAD", key, &[], &[])?;
            let result = ureq::head(&signed.url)
                .header("host", &self.host)
                .header("x-amz-content-sha256", signed.payload)
                .header("x-amz-date", signed.time)
                .header("authorization", signed.authorization)
                .call();
            match result {
                Ok(_) => {
                    let outcome = Outcome::Held(());
                    note("s3.head", &outcome, attempt + 1, started);
                    return Ok(outcome);
                }
                Err(ureq::Error::StatusCode(404)) => {
                    let outcome = Outcome::Missing;
                    note("s3.head", &outcome, attempt + 1, started);
                    return Ok(outcome);
                }
                Err(error) if retryable(&error) && attempt + 1 < ATTEMPTS => continue,
                Err(error) => {
                    fail("s3.head", &error, attempt + 1, started);
                    return Err(format!("cannot head bucket object {key}: {error}"));
                }
            }
        }
        unreachable!()
    }
}

pub(super) fn note<T>(
    operation: &'static str,
    outcome: &Outcome<T>,
    count: usize,
    started: Instant,
) {
    let (label, code) = match outcome {
        Outcome::Held(_) => ("ok", Some(200)),
        Outcome::Missing => ("missing", Some(404)),
        Outcome::Stale => ("stale", Some(412)),
    };
    exchange::attempt(operation, label, code, (count as u8, started.elapsed()));
}

pub(super) fn fail(operation: &'static str, error: &ureq::Error, count: usize, started: Instant) {
    let code = match error {
        ureq::Error::StatusCode(code) => Some(*code),
        _ => None,
    };
    exchange::attempt(operation, "error", code, (count as u8, started.elapsed()));
}

pub(super) fn retryable(error: &ureq::Error) -> bool {
    match error {
        ureq::Error::StatusCode(code) => matches!(code, 408 | 429 | 500..=599),
        ureq::Error::Io(_)
        | ureq::Error::Protocol(_)
        | ureq::Error::Timeout(_)
        | ureq::Error::HostNotFound
        | ureq::Error::ConnectionFailed => true,
        _ => false,
    }
}

pub(super) fn pause(attempt: usize) {
    if attempt > 0 {
        std::thread::sleep(Duration::from_millis(200 * attempt as u64));
    }
}
