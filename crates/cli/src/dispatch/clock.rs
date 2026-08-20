use std::time::{SystemTime, UNIX_EPOCH};

pub fn ahead(minutes: u64) -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("cannot read the clock: {error}"))?
        .as_secs();
    Ok(stamp(now + minutes * 60))
}

pub fn mark() -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("cannot read the clock: {error}"))?
        .as_secs();
    let rest = now % 86_400;
    let (year, month, day) = civil((now / 86_400) as i64);
    Ok(format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    ))
}

fn stamp(seconds: u64) -> String {
    let rest = seconds % 86_400;
    let (year, month, day) = civil((seconds / 86_400) as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}

fn civil(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let doe = shifted.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}
