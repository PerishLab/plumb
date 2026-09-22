use plumb::land::rejoin::latest;
use std::path::Path;
use std::process::Command;

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
            "stable {} at {} is not an ancestor of main; run plumb release rejoin, and plumb release owed for every obligation it leaves",
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
