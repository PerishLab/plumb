use super::course::Course;
use super::wharf::launch::{Launch, launch};
use crate::shape::release::Spec;
use clap::Args;
use plumb::depot::v3::{Generation, Kind, Query};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

mod goods;
mod yard;

use goods::{Consignment, Good, SCHEMA};

const WORKFLOW: &str = "depot.yml";

#[derive(Args)]
#[group(skip)]
pub struct Consign {
    #[arg(long)]
    version: String,
    #[arg(long, value_parser = ["changelog", "skill"])]
    kind: String,
    #[arg(long, value_name = "DIR")]
    dir: Option<PathBuf>,
    #[arg(long = "no-watch")]
    unwatched: bool,
    #[arg(long = "dry-run")]
    dry: bool,
}

struct Lodging<'a> {
    spec: &'a Spec,
    channel: &'a str,
    marker: &'a str,
    kind: Kind,
}

pub(super) fn consign(options: Consign) -> Result<String, String> {
    let root = super::worktree::root()?;
    let marker = if options.version.starts_with('v') {
        options.version.clone()
    } else {
        format!("v{}", options.version)
    };
    let channel = super::super::release::channel(&marker)?;
    let spec = Spec::controller(&root)?;
    let origin = text(&root, &["remote", "get-url", "origin"])?;
    let repository = super::wharf::repository(&origin)?;
    stands(&root, &marker)?;
    let (objects, kind) = gathered(&options, &root, &spec, &marker)?;
    let (body, digest) = Consignment {
        schema: SCHEMA,
        repository: &repository,
        marker: &marker,
        kind: &options.kind,
        objects: &objects,
    }
    .encode()?;
    let key = format!("{repository}/{marker}/{}/{digest}.json", options.kind);
    let mut course = Course::new(options.dry);
    let placed = course.step(format!("PUT yard {key} (if-none-match)"), || {
        yard::write(&key, &body)
    })?;
    let fields = [
        format!("repository={repository}"),
        format!("marker={marker}"),
        format!("kind={}", options.kind),
        format!("digest={digest}"),
    ];
    let watch = !options.unwatched;
    let launched = launch(
        &mut course,
        Launch {
            workflow: WORKFLOW,
            fields: &fields,
            watch,
        },
    )?;
    let (Some(placed), Some(url)) = (placed, launched) else {
        return Ok(course.plan());
    };
    if !watch {
        return Ok(format!("{placed} {digest}; lodging at {url}"));
    }
    let lodging = Lodging {
        spec: &spec,
        channel: &channel,
        marker: &marker,
        kind,
    };
    lodged(&lodging, &objects).map(|generation| {
        format!(
            "{placed} {digest}; lodged {} generation {generation} for {marker} through {url}",
            options.kind
        )
    })
}

fn gathered(
    options: &Consign,
    root: &Path,
    spec: &Spec,
    marker: &str,
) -> Result<(Vec<Good>, Kind), String> {
    if options.kind == "skill" {
        if !spec.skill {
            return Err(format!("{} declares no skill to consign", spec.product));
        }
        let dir = options
            .dir
            .as_deref()
            .ok_or("a skill consigns the brief in --dir")?;
        brief(dir)?;
        return Ok((goods::directory(dir)?, Kind::Skill));
    }
    let dir = options
        .dir
        .as_deref()
        .ok_or("a changelog consigns the notes in --dir")?;
    crate::command::changelog::prove(root, dir, marker)?;
    Ok((goods::directory(dir)?, Kind::Changelog))
}

fn brief(dir: &Path) -> Result<(), String> {
    let rule = crate::catalog::member::parse(WAYFINDER)
        .and_then(|held| crate::catalog::member::member(&held))?;
    vet(dir, rule.leaf.as_deref().unwrap_or("SKILL.md"), rule.bytes)
}

const WAYFINDER: &str = "rule://seat/wayfinder";

fn vet(dir: &Path, leaf: &str, cap: Option<usize>) -> Result<(), String> {
    let path = dir.join(leaf);
    let held = std::fs::metadata(&path)
        .map_err(|_| format!("{} carries no {leaf}; a skill is a brief", dir.display()))?;
    if let Some(bytes) = cap
        && held.len() > bytes as u64
    {
        return Err(format!(
            "{} carries {} bytes where {WAYFINDER} caps {bytes}; see: plumb cookbook wayfinder",
            path.display(),
            held.len()
        ));
    }
    Ok(())
}

fn lodged(lodging: &Lodging<'_>, objects: &[Good]) -> Result<String, String> {
    let source = super::owed::source(lodging.spec);
    let found = Generation::latest(Query {
        source: &source,
        product: &lodging.spec.product,
        channel: lodging.channel,
        version: lodging.marker,
        kind: lodging.kind,
    })?
    .ok_or_else(|| {
        format!(
            "{source} holds no {} for {} after the lodge",
            lodging.kind.label(),
            lodging.marker
        )
    })?;
    let standing = found
        .manifest
        .objects
        .iter()
        .map(|held| (held.path.clone(), held.sha256.clone(), held.executable))
        .collect::<BTreeSet<_>>();
    let wanted = objects
        .iter()
        .map(|held| (held.path.clone(), held.sha256.clone(), held.executable))
        .collect::<BTreeSet<_>>();
    if standing != wanted {
        return Err(format!(
            "the standing {} generation {} does not hold what was consigned",
            lodging.kind.label(),
            found.pointer.generation
        ));
    }
    Ok(found.pointer.generation)
}

fn stands(root: &Path, marker: &str) -> Result<(), String> {
    let reference = format!("refs/tags/{marker}");
    if text(root, &["rev-parse", "--verify", "-q", &reference]).is_ok() {
        return Ok(());
    }
    text(
        root,
        &[
            "fetch",
            "--no-tags",
            "origin",
            &format!("{reference}:{reference}"),
        ],
    )
    .map(|_| ())
    .map_err(|_| format!("origin holds no marker {marker}; consign describes a stamped release"))
}

fn text(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    Err(format!(
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

#[cfg(all(test, unix))]
mod proof;
