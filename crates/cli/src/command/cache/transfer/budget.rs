use std::time::Duration;

pub(super) const DEFAULT: u64 = 60;
const KEY: &str = "PLUMB_GUARD_RESTORE_BUDGET";

pub(crate) fn budget() -> Result<Duration, String> {
    decide(plumb::config::value(KEY).as_deref())
}

pub(super) fn decide(value: Option<&str>) -> Result<Duration, String> {
    match value {
        None => Ok(Duration::from_secs(DEFAULT)),
        Some(value) => value
            .parse()
            .ok()
            .filter(|seconds| *seconds != 0)
            .map(Duration::from_secs)
            .ok_or_else(|| format!("{KEY} must be a positive number of seconds, not {value}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget() {
        assert_eq!(decide(None).unwrap(), Duration::from_secs(DEFAULT));
        assert_eq!(decide(Some("90")).unwrap(), Duration::from_secs(90));
        for value in ["0", "", "30s", "auto", "-1"] {
            let error = decide(Some(value)).unwrap_err();
            assert!(
                error
                    .starts_with("PLUMB_GUARD_RESTORE_BUDGET must be a positive number of seconds"),
                "{value}: {error}"
            );
        }
    }
}
