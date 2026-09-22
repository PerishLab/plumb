use crate::shape::pair::rejoin;
use plumb::land::rejoin::{latest, tags};
use std::path::Path;

pub(super) fn require(root: &Path, remote: &str, listing: &str) -> Result<(), String> {
    let Some(stable) = latest(tags(listing)) else {
        return Ok(());
    };
    let main = super::reference(listing, "refs/heads/main").ok_or_else(|| {
        format!("{remote} has no main; a stable marker settles into main before the next")
    })?;
    super::text(
        "fetch main and the standing stable marker",
        super::git(
            root,
            &[
                "fetch",
                "--no-tags",
                remote,
                "refs/heads/main",
                &format!("refs/tags/{}", stable.marker),
            ],
        )?,
    )?;
    if rejoin::settled(root, &stable.commit, &main) {
        return Ok(());
    }
    Err(format!(
        "stable {} at {} is not an ancestor of {remote} main; run plumb release rejoin before stamping another stable marker",
        stable.marker, stable.commit
    ))
}
