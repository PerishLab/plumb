use super::Dispatch;
use super::course::Course;
use super::value;
use std::path::Path;
use std::process::{Command, Output};

pub(super) mod launch;

const HUB: &str = "PerishLab/wharf";
const WORKFLOW: &str = "ship.yml";
const PRERELEASES: [&str; 2] = ["rc", "beta"];
const STABLE: &str = "stable";

pub(in crate::command) fn stamp(raw: &str, remote: &str, dry: bool) -> Result<String, String> {
    let held = if raw.starts_with('v') {
        raw.to_string()
    } else {
        format!("v{raw}")
    };
    let channel = super::super::release::channel(&held)?;
    if channel != STABLE && !PRERELEASES.contains(&channel.as_str()) {
        return Err(format!(
            "{held} is not an rc, beta or {STABLE} marker; wharf distribution stamps only those"
        ));
    }
    let version = value::version(&held, &channel)?;
    let base = version.split('-').next().unwrap_or(&version).to_string();
    let branch = value::branch(&base);
    let root = super::worktree::root()?;
    let url = text(
        "read the remote",
        git(&root, &["remote", "get-url", remote])?,
    )?;
    repository(&url)?;
    let listing = text(
        "list the remote",
        git(&root, &["ls-remote", "--heads", "--tags", remote])?,
    )?;
    let spec = crate::shape::release::Spec::controller(&root)?;
    super::owed::Seat {
        root: &root,
        remote,
        listing: &listing,
        spec: &spec,
    }
    .require(&version)?;
    let head = reference(&listing, &format!("refs/heads/{branch}")).ok_or_else(|| {
        format!("{remote} has no {branch}; a marker stands only on its release line")
    })?;
    let message = if channel == STABLE {
        if reference(&listing, &format!("refs/tags/{version}")).is_some() {
            return Err(format!(
                "{version} already stands; a stable marker never moves"
            ));
        }
        let authority = super::super::release::authority(&root)?;
        let promoted = promoted(&listing, &base, &head, |channel, marker| {
            plumb::bucket::fetch(&super::owed::distribution(&authority, channel, marker))
        })?;
        format!("{version}\n\npromotes {promoted}")
    } else {
        let expected = next(&listing, &base, &channel);
        if version != format!("{base}-{channel}.{expected}") {
            return Err(format!(
                "{version} skips the line; the next {channel} marker on {branch} is {base}-{channel}.{expected}"
            ));
        }
        version.clone()
    };
    let mut course = Course::new(dry);
    course.step(format!("fetch {branch} from {remote}"), || {
        text(
            "fetch the release line",
            git(&root, &["fetch", remote, &format!("refs/heads/{branch}")])?,
        )
    })?;
    course.step(format!("git tag -a {version} {head}"), || {
        text(
            "stamp the marker",
            git(&root, &["tag", "-a", &version, &head, "-m", &message])?,
        )
    })?;
    course.step(format!("git push {remote} refs/tags/{version}"), || {
        text(
            "publish the marker",
            git(&root, &["push", remote, &format!("refs/tags/{version}")])?,
        )
    })?;
    if course.dry() {
        return Ok(course.plan());
    }
    let promotes = message
        .lines()
        .last()
        .filter(|line| line.starts_with("promotes "))
        .map(|line| format!(", {line}"))
        .unwrap_or_default();
    Ok(format!("stamped {version} at {head} on {branch}{promotes}"))
}

fn promoted(
    listing: &str,
    base: &str,
    head: &str,
    read: impl Fn(&str, &str) -> Result<Option<Vec<u8>>, String>,
) -> Result<String, String> {
    let mut held = Vec::new();
    for channel in PRERELEASES {
        let prefix = format!("refs/tags/{base}-{channel}.");
        let mut numbers = listing
            .lines()
            .filter_map(|line| {
                let (object, name) = line.split_once('\t')?;
                let number = name
                    .strip_prefix(&prefix)?
                    .strip_suffix("^{}")?
                    .parse::<u64>()
                    .ok()?;
                (object == head).then_some(number)
            })
            .collect::<Vec<_>>();
        numbers.sort_unstable_by(|left, right| right.cmp(left));
        held.extend(
            numbers
                .into_iter()
                .map(|number| (channel, format!("{base}-{channel}.{number}"))),
        );
    }
    if held.is_empty() {
        return Err(format!(
            "no rc or beta marker stands at {head}; a stable marker promotes a shipped prerelease"
        ));
    }
    for (channel, marker) in held {
        if super::owed::complete(read(channel, &marker)?, &marker, head)? {
            return Ok(marker);
        }
    }
    Err(format!(
        "no prerelease marker at {head} has completed its distribution; ship one before promoting it"
    ))
}

pub(super) fn dispatch(options: Dispatch) -> Result<String, String> {
    super::super::release::channel(&options.marker)?;
    let root = super::worktree::root()
        .map_err(|error| format!("ship dispatch runs in the product's repository: {error}"))?;
    let origin = repository(&text(
        "read the remote",
        git(&root, &["remote", "get-url", "origin"])?,
    )?)?;
    let repo = if options.repo.is_empty() {
        origin.clone()
    } else {
        options.repo.clone()
    };
    if repo != origin {
        return Err(format!(
            "--repo {repo} is not this repository's origin {origin}; dispatch runs in the product's repository"
        ));
    }
    let listing = text(
        "list the remote",
        git(&root, &["ls-remote", "--heads", "--tags", "origin"])?,
    )?;
    if reference(&listing, &format!("refs/tags/{}", options.marker)).is_none() {
        return Err(format!("{repo} has no marker {}", options.marker));
    }
    let spec = crate::shape::release::Spec::controller(&root)?;
    super::owed::Seat {
        root: &root,
        remote: "origin",
        listing: &listing,
        spec: &spec,
    }
    .require(&options.marker)?;
    let mut course = Course::new(options.dry);
    let fields = [
        format!("repository={repo}"),
        format!("marker={}", options.marker),
    ];
    let launched = launch::launch(
        &mut course,
        launch::Launch {
            workflow: WORKFLOW,
            fields: &fields,
            watch: options.watch,
        },
    )?;
    match launched {
        None => Ok(course.plan()),
        Some(url) if options.watch => Ok(format!("{url} succeeded")),
        Some(url) => Ok(url),
    }
}

pub(super) fn repository(url: &str) -> Result<String, String> {
    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))
        .ok_or_else(|| {
            format!(
                "{url} is not a GitHub remote; markers are stamped where wharf distributes from"
            )
        })?;
    Ok(path.trim_end_matches(".git").to_string())
}

pub(super) fn reference(listing: &str, name: &str) -> Option<String> {
    listing.lines().find_map(|line| {
        let (object, held) = line.split_once('\t')?;
        (held == name).then(|| object.to_string())
    })
}

fn next(listing: &str, base: &str, channel: &str) -> u64 {
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

pub(super) fn git(root: &Path, args: &[&str]) -> Result<Output, String> {
    run(Command::new("git").args(args).current_dir(root))
}

fn run(command: &mut Command) -> Result<Output, String> {
    command
        .output()
        .map_err(|error| format!("cannot run {:?}: {error}", command.get_program()))
}

pub(super) fn text(deed: &str, output: Output) -> Result<String, String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "cannot {deed}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod proof;
