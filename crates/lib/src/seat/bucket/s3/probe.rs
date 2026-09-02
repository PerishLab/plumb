use super::{ATTEMPTS, Control, Outcome, pause, retryable};

impl Control {
    pub fn head(&self, key: &str) -> Result<Outcome<()>, String> {
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
                Ok(_) => return Ok(Outcome::Held(())),
                Err(ureq::Error::StatusCode(404)) => return Ok(Outcome::Missing),
                Err(error) if retryable(&error) && attempt + 1 < ATTEMPTS => continue,
                Err(error) => return Err(format!("cannot head bucket object {key}: {error}")),
            }
        }
        unreachable!()
    }
}
