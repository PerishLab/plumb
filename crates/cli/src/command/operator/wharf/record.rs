use plumb::seat::release::Distribution;

pub(super) fn unmoved(
    record: Option<&Distribution>,
    marker: &str,
    commit: &str,
) -> Result<(), String> {
    match record {
        Some(held) if held.commit != commit => Err(format!(
            "{marker} already holds a distribution record for {}; a marker never moves, so stamp the next marker instead",
            held.commit
        )),
        _ => Ok(()),
    }
}

pub(super) fn next(listing: &str, base: &str, channel: &str) -> u64 {
    let prefix = format!("refs/tags/{base}-{channel}.");
    listing
        .lines()
        .filter_map(|line| {
            line.split_once('\t')?
                .1
                .strip_prefix(&prefix)?
                .parse::<u64>()
                .ok()
        })
        .max()
        .unwrap_or(0)
        + 1
}
