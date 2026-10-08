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
