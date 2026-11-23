pub(super) const DEFAULT: u64 = 25;
const KEY: &str = "PLUMB_GUARD_RESTORE_PAGES";

pub(crate) fn pages() -> Result<u64, String> {
    decide(plumb::config::value(KEY).as_deref())
}

pub(super) fn decide(value: Option<&str>) -> Result<u64, String> {
    match value {
        None => Ok(DEFAULT),
        Some(value) => value
            .parse()
            .ok()
            .filter(|pages| *pages != 0)
            .ok_or_else(|| format!("{KEY} must be a positive page count, not {value}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages() {
        assert_eq!(decide(None).unwrap(), DEFAULT);
        assert_eq!(decide(Some("5")).unwrap(), 5);
        assert_eq!(decide(Some("25")).unwrap(), 25);
        for value in ["0", "", "25p", "auto", "-1"] {
            let error = decide(Some(value)).unwrap_err();
            assert!(
                error.starts_with("PLUMB_GUARD_RESTORE_PAGES must be a positive page count"),
                "{value}: {error}"
            );
        }
    }
}
