use super::Dispatch;
use super::course::Course;
use super::value;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const HUB: &str = "PerishLab/wharf";
const WORKFLOW: &str = "ship.yml";
const CHANNEL: &str = "beta";

pub(super) fn stamp(raw: &str, remote: &str, dry: bool) -> Result<String, String> {
    let held = if raw.starts_with('v') { raw.to_string() } else { format!("v{raw}") };
    let channel = super::super::release::channel(&held)?;
    if channel != CHANNEL {
        return Err(format!("{held} is not a {CHANNEL} marker; wharf distribution stamps {CHANNEL} markers only"));
    }
    let version = value::version(&held, &channel)?;
    let base = version.split('-').next().unwrap_or(&version).to_string();
    let branch = value::branch(&base);
    let root = plumb::forgejo::git::root()?;
    let url = text("read the remote", git(&root, &["remote", "get-url", remote])?)?;
    repository(&url)?;
    let listing = text("list the remote", git(&root, &["ls-remote", "--heads", "--tags", remote])?)?;
    let head = reference(&listing, &format!("refs/heads/{branch}"))
        .ok_or_else(|| format!("{remote} has no {branch}; a marker stands only on its release line"))?;
    let expected = next(&listing, &base);
    if version != format!("{base}-{CHANNEL}.{expected}") {
        return Err(format!("{version} skips the line; the next {CHANNEL} marker on {branch} is {base}-{CHANNEL}.{expected}"));
    }
    let mut course = Course::new(dry);
    course.step(format!("fetch {branch} from {remote}"), || {
        text("fetch the release line", git(&root, &["fetch", remote, &format!("refs/heads/{branch}")])?)
    })?;
    course.step(format!("git tag -a {version} {head}"), || {
        text("stamp the marker", git(&root, &["tag", "-a", &version, &head, "-m", &version])?)
    })?;
    course.step(format!("git push {remote} refs/tags/{version}"), || {
        text("publish the marker", git(&root, &["push", remote, &format!("refs/tags/{version}")])?)
    })?;
    if course.dry() {
        return Ok(course.plan());
    }
    Ok(format!("stamped {version} at {head} on {branch}"))
}

pub(super) fn dispatch(options: Dispatch) -> Result<String, String> {
    super::super::release::channel(&options.marker)?;
    if options.repo.split('/').count() != 2 || options.repo.split('/').any(str::is_empty) {
        return Err("ship dispatch requires --repo owner/name".into());
    }
    let product = format!("https://github.com/{}", options.repo);
    let marker = format!("refs/tags/{}", options.marker);
    let listing = text("read the product marker", run(Command::new("git").args(["ls-remote", &product, &marker]))?)?;
    if reference(&listing, &marker).is_none() {
        return Err(format!("{} has no marker {}", options.repo, options.marker));
    }
    let mut course = Course::new(options.dry);
    let launched = course.step(format!("gh workflow run {WORKFLOW} -R {HUB} -f repository={} -f marker={}", options.repo, options.marker), || {
        let fields = [format!("repository={}", options.repo), format!("marker={}", options.marker)];
        text("dispatch wharf", run(Command::new("gh").args(["workflow", "run", WORKFLOW, "-R", HUB, "-f", &fields[0], "-f", &fields[1]]))?)
    })?;
    let Some(said) = launched else {
        return Ok(course.plan());
    };
    let url = said.lines().find(|line| line.contains("/actions/runs/")).ok_or_else(|| format!("wharf dispatch named no run: {said}"))?.trim().to_string();
    if !options.watch {
        return Ok(url);
    }
    let id = url.rsplit('/').next().unwrap_or_default().to_string();
    let status = Command::new("gh").args(["run", "watch", &id, "-R", HUB, "--exit-status"]).stdout(Stdio::inherit()).stderr(Stdio::inherit()).status().map_err(|error| format!("cannot run gh: {error}"))?;
    if status.success() { Ok(format!("{url} succeeded")) } else { Err(format!("{url} did not succeed")) }
}

fn repository(url: &str) -> Result<String, String> {
    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))
        .ok_or_else(|| format!("{url} is not a GitHub remote; markers are stamped where wharf distributes from"))?;
    Ok(path.trim_end_matches(".git").to_string())
}

fn reference(listing: &str, name: &str) -> Option<String> {
    listing.lines().find_map(|line| {
        let (object, held) = line.split_once('\t')?;
        (held == name).then(|| object.to_string())
    })
}

fn next(listing: &str, base: &str) -> u64 {
    let prefix = format!("refs/tags/{base}-{CHANNEL}.");
    listing
        .lines()
        .filter_map(|line| line.split_once('\t')?.1.strip_prefix(&prefix)?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        + 1
}

fn git(root: &Path, args: &[&str]) -> Result<Output, String> {
    run(Command::new("git").args(args).current_dir(root))
}

fn run(command: &mut Command) -> Result<Output, String> {
    command.output().map_err(|error| format!("cannot run {:?}: {error}", command.get_program()))
}

fn text(deed: &str, output: Output) -> Result<String, String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!("cannot {deed}: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::{next, reference, repository};

    const LISTING: &str = "aaa\trefs/heads/release/v0.38.0\nbbb\trefs/tags/v0.38.0-beta.1\nccc\trefs/tags/v0.38.0-beta.1^{}\nddd\trefs/tags/v0.38.0-beta.3\neee\trefs/tags/v0.38.1-beta.9\n";

    #[test]
    fn next_beta_follows_the_highest_on_the_line() {
        assert_eq!(next(LISTING, "v0.38.0"), 4);
        assert_eq!(next(LISTING, "v0.39.0"), 1);
    }

    #[test]
    fn reference_reads_exact_names() {
        assert_eq!(reference(LISTING, "refs/heads/release/v0.38.0").as_deref(), Some("aaa"));
        assert_eq!(reference(LISTING, "refs/tags/v0.38.0-beta.1").as_deref(), Some("bbb"));
        assert_eq!(reference(LISTING, "refs/heads/release/v0.37.0"), None);
    }

    #[test]
    fn repository_accepts_only_github_remotes() {
        assert_eq!(repository("https://github.com/PerishLab/plumb.git").unwrap(), "PerishLab/plumb");
        assert_eq!(repository("git@github.com:PerishLab/plumb.git").unwrap(), "PerishLab/plumb");
        assert!(repository("ssh://git@git.perish.top/PerishLab/plumb.git").is_err());
    }
}
