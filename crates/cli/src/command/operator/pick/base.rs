use super::{command, text};
use std::path::Path;

pub(crate) fn require(root: &Path, base: &str) -> Result<(), String> {
    if super::super::topology::ancestor(root, base, "origin/main")? {
        return Ok(());
    }
    let tags = text(
        "inspect release base markers",
        command(root, ["tag", "--points-at", base])?,
    )?;
    for tag in tags.lines() {
        if !semver::Version::parse(tag.trim_start_matches('v'))
            .is_ok_and(|version| version.pre.is_empty())
        {
            continue;
        }
        let marker = crate::command::release::ReleaseMarker::bound(root, tag)?;
        if marker.channel == "stable" && marker.commit == base {
            return Ok(());
        }
    }
    Err(format!(
        "release base {base} must be held by origin/main or an exact verified stable marker"
    ))
}

pub(super) fn resolve(root: &Path, release: &str, version: &str) -> Result<String, String> {
    let listed = text(
        "inspect release preparation",
        command(
            root,
            ["log", "--first-parent", "--format=%H%x1f%B%x00", release],
        )?,
    )?;
    for record in listed.split('\0').rev() {
        let Some((commit, body)) = record.trim_start().split_once('\u{1f}') else {
            continue;
        };
        let title = body.lines().next().unwrap_or("");
        if title != format!("Prepare {version}")
            && title != format!("Record the datum {version} judges against")
        {
            continue;
        }
        let base = text(
            "resolve preparation parent",
            command(root, ["rev-parse", &format!("{commit}^")])?,
        )?;
        let prepared = super::super::version::prepared(super::super::version::Preparation {
            root,
            commit,
            base: &base,
            version,
            body: body.trim(),
        });
        let datum = title == format!("Record the datum {version} judges against")
            && super::super::datum::Seat(root).carried(commit, version);
        if !prepared && !datum {
            return Err(format!(
                "{release} contains a commit without cherry-pick -x provenance: invalid preparation {commit}"
            ));
        }
        require(root, &base)?;
        return Ok(base);
    }
    Err(format!(
        "{release} has no verifiable version preparation base"
    ))
}
