use std::path::Path;
use std::process::Command;

pub struct Stable {
    pub marker: String,
    pub commit: String,
}

pub fn latest<'a>(tags: impl IntoIterator<Item = (&'a str, &'a str)>) -> Option<Stable> {
    tags.into_iter()
        .filter_map(|(name, commit)| {
            let version = semver::Version::parse(name.strip_prefix('v')?).ok()?;
            version.pre.is_empty().then_some((version, name, commit))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, name, commit)| Stable {
            marker: name.to_string(),
            commit: commit.to_string(),
        })
}

pub fn settled(root: &Path, commit: &str, main: &str) -> bool {
    git(root, &["merge-base", "--is-ancestor", commit, main])
        .status()
        .is_ok_and(|status| status.success())
}

pub fn unsettled(root: &Path) -> Option<String> {
    let main = text(git(
        root,
        &["rev-parse", "--verify", "-q", "refs/heads/main"],
    ))?;
    let listing = text(git(
        root,
        &[
            "for-each-ref",
            "--format=%(refname:strip=2)%09%(*objectname)%09%(objectname)",
            "refs/tags/v*",
        ],
    ))?;
    let tags = listing.lines().filter_map(|line| {
        let mut fields = line.split('\t');
        let name = fields.next()?;
        let peeled = fields.next()?;
        let object = fields.next()?;
        Some((name, if peeled.is_empty() { object } else { peeled }))
    });
    let stable = latest(tags)?;
    (!settled(root, &stable.commit, &main)).then(|| {
        format!(
            "stable {} at {} is not an ancestor of main; merge it home before the next stable marker",
            stable.marker,
            &stable.commit[..stable.commit.len().min(12)]
        )
    })
}

fn git(root: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(args);
    command
}

fn text(mut command: Command) -> Option<String> {
    let output = command.output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}
