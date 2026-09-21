use crate::shape::pair::rejoin;
use std::path::Path;

pub(super) fn require(root: &Path, remote: &str, listing: &str) -> Result<(), String> {
    let Some(stable) = rejoin::latest(tags(listing)) else {
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
        "stable {} at {} is not an ancestor of {remote} main; merge it home before stamping another stable marker",
        stable.marker, stable.commit
    ))
}

pub(super) fn tags(listing: &str) -> Vec<(&str, &str)> {
    let mut held = std::collections::BTreeMap::new();
    for line in listing.lines() {
        let Some((object, name)) = line.split_once('\t') else {
            continue;
        };
        let Some(name) = name.strip_prefix("refs/tags/") else {
            continue;
        };
        match name.strip_suffix("^{}") {
            Some(peeled) => {
                held.insert(peeled, object);
            }
            None => {
                held.entry(name).or_insert(object);
            }
        }
    }
    held.into_iter().collect()
}
