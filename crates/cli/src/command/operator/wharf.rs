use super::Dispatch;
use super::course::Course;
use super::value;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const HUB: &str = "PerishLab/wharf";
const WORKFLOW: &str = "ship.yml";
const CHANNEL: &str = "beta";
const STABLE: &str = "stable";

pub(super) fn stamp(raw: &str, remote: &str, dry: bool) -> Result<String, String> {
    let held = if raw.starts_with('v') { raw.to_string() } else { format!("v{raw}") };
    let channel = super::super::release::channel(&held)?;
    if channel != CHANNEL && channel != STABLE {
        return Err(format!("{held} is neither a {CHANNEL} nor a {STABLE} marker; wharf distribution stamps only those"));
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
    let message = if channel == STABLE {
        if reference(&listing, &format!("refs/tags/{version}")).is_some() {
            return Err(format!("{version} already stands; a stable marker never moves"));
        }
        let authority = super::super::release::authority(&root)?;
        let promoted = promoted(&listing, &base, &head, |beta| {
            plumb::bucket::fetch(&format!("{}/v1/releases/{CHANNEL}/{beta}/seal.json", authority.trim_end_matches('/')))
        })?;
        format!("{version}\n\npromotes {promoted}")
    } else {
        let expected = next(&listing, &base);
        if version != format!("{base}-{CHANNEL}.{expected}") {
            return Err(format!("{version} skips the line; the next {CHANNEL} marker on {branch} is {base}-{CHANNEL}.{expected}"));
        }
        version.clone()
    };
    let mut course = Course::new(dry);
    course.step(format!("fetch {branch} from {remote}"), || {
        text("fetch the release line", git(&root, &["fetch", remote, &format!("refs/heads/{branch}")])?)
    })?;
    course.step(format!("git tag -a {version} {head}"), || {
        text("stamp the marker", git(&root, &["tag", "-a", &version, &head, "-m", &message])?)
    })?;
    course.step(format!("git push {remote} refs/tags/{version}"), || {
        text("publish the marker", git(&root, &["push", remote, &format!("refs/tags/{version}")])?)
    })?;
    if course.dry() {
        return Ok(course.plan());
    }
    let promotes = message.lines().last().filter(|line| line.starts_with("promotes ")).map(|line| format!(", {line}")).unwrap_or_default();
    Ok(format!("stamped {version} at {head} on {branch}{promotes}"))
}

fn promoted(
    listing: &str,
    base: &str,
    head: &str,
    seal: impl Fn(&str) -> Result<Option<Vec<u8>>, String>,
) -> Result<String, String> {
    let prefix = format!("refs/tags/{base}-{CHANNEL}.");
    let mut held = listing
        .lines()
        .filter_map(|line| {
            let (object, name) = line.split_once('\t')?;
            let number = name.strip_prefix(&prefix)?.strip_suffix("^{}")?.parse::<u64>().ok()?;
            (object == head).then_some(number)
        })
        .collect::<Vec<_>>();
    held.sort_unstable_by(|left, right| right.cmp(left));
    if held.is_empty() {
        return Err(format!("no {CHANNEL} marker stands at {head}; a stable marker promotes a shipped {CHANNEL}"));
    }
    for number in held {
        let beta = format!("{base}-{CHANNEL}.{number}");
        let Some(body) = seal(&beta)? else { continue };
        let document: serde_json::Value = serde_json::from_slice(&body).map_err(|error| format!("{beta} seal does not parse: {error}"))?;
        if document["releaseVersion"] == beta.as_str() && document["channel"] == CHANNEL && document["commit"] == head {
            return Ok(beta);
        }
    }
    Err(format!("no {CHANNEL} marker at {head} has a published seal; ship one before promoting it"))
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
    use super::{next, promoted, reference, repository};

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

    const PROMOTION: &str = "aaa\trefs/heads/release/v0.38.0\nt1\trefs/tags/v0.38.0-beta.15\nbbb\trefs/tags/v0.38.0-beta.15^{}\nt2\trefs/tags/v0.38.0-beta.16\naaa\trefs/tags/v0.38.0-beta.16^{}\nt3\trefs/tags/v0.38.0-beta.17\naaa\trefs/tags/v0.38.0-beta.17^{}\n";

    fn seal(beta: &str, commit: &str) -> Vec<u8> {
        format!(r#"{{"releaseVersion":"{beta}","channel":"beta","commit":"{commit}"}}"#).into_bytes()
    }

    #[test]
    fn stable_promotes_the_highest_shipped_beta_at_the_head() {
        let found = promoted(PROMOTION, "v0.38.0", "aaa", |beta| {
            Ok((beta == "v0.38.0-beta.16").then(|| seal(beta, "aaa")))
        });
        assert_eq!(found.as_deref(), Ok("v0.38.0-beta.16"));
    }

    #[test]
    fn stable_refuses_without_a_shipped_beta_at_the_head() {
        assert!(promoted(PROMOTION, "v0.38.0", "ccc", |_| Ok(None)).unwrap_err().contains("no beta marker stands"));
        assert!(promoted(PROMOTION, "v0.38.0", "aaa", |_| Ok(None)).unwrap_err().contains("has a published seal"));
        let moved = promoted(PROMOTION, "v0.38.0", "aaa", |beta| Ok(Some(seal(beta, "bbb"))));
        assert!(moved.unwrap_err().contains("has a published seal"));
    }
}
