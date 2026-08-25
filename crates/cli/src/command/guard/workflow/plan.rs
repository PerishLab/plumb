use super::tree;
use crate::shape;
use clap::Args;
use plumb::cli::Root;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Args)]
pub struct Input {
    #[arg(long)]
    base: Option<String>,
    #[arg(long = "world")]
    world: Vec<String>,
    #[command(flatten)]
    target: Root,
}

pub fn run(input: Input) -> i32 {
    let root = Path::new(&input.target.root);
    match render(root, input.base.as_deref(), &input.world) {
        Ok(plan) => {
            println!("{plan}");
            0
        }
        Err(error) => {
            eprintln!("plumb workflow plan: {error}");
            1
        }
    }
}

fn render(root: &Path, base: Option<&str>, entries: &[String]) -> Result<String, String> {
    let mut current = shape::workflow::read(root);
    if let Some(error) = current.refusal {
        return Err(error);
    }
    let git = tree::Git::new(root);
    let mut prior = match base {
        Some(base) => git
            .file(base, "plumb.toml")?
            .map(|text| shape::workflow::parse(&text))
            .unwrap_or_default(),
        None => shape::workflow::Held::default(),
    };
    if let Some(error) = prior.refusal {
        return Err(format!("base {}: {error}", base.unwrap_or_default()));
    }
    let world = world(entries)?;
    let before = match base {
        Some(base) => tree::Tree::read(root, Some(base))?,
        None => tree::Tree::default(),
    };
    let after = tree::Tree::read(root, None)?;
    if current.keys.is_empty() {
        current = defaults(&after);
    }
    if prior.keys.is_empty() {
        prior = defaults(&before);
    }
    if current.keys.is_empty() {
        return Err("the repository implies no workflow action".to_string());
    }
    let base = base.map(|base| git.revision(base)).transpose()?;
    let head = git.revision("HEAD")?;
    let actions: Vec<Action> = current
        .keys
        .iter()
        .map(|key| {
            let prior = prior.keys.iter().find(|held| held.name() == key.name());
            let prior = prior.map(|held| before.digest(held));
            let current = after.digest(key);
            let run = prior.as_ref() != Some(&current);
            Action {
                name: key.name(),
                run,
                reason: match (&prior, run) {
                    (None, _) => "action-added",
                    (Some(_), true) => "input-moved",
                    (Some(_), false) => "input-held",
                },
                before: prior,
                after: current,
                reuse: Reuse {
                    kind: "none",
                    source: "",
                },
            }
        })
        .collect();
    let identity = digest(&world, &actions);
    serde_json::to_string(&Plan {
        schema: "plumb.workflow-plan/v1",
        base,
        head,
        identity,
        world,
        actions,
    })
    .map_err(|error| format!("cannot encode the plan: {error}"))
}

fn defaults(tree: &tree::Tree) -> shape::workflow::Held {
    shape::workflow::inferred(
        tree.has("Cargo.toml"),
        tree.has("pnpm-lock.yaml"),
        tree.has("plumb.toml"),
        tree.has("ectropy.toml"),
    )
}

#[derive(Serialize)]
struct Plan {
    schema: &'static str,
    base: Option<String>,
    head: String,
    world: BTreeMap<String, String>,
    identity: String,
    actions: Vec<Action>,
}

#[derive(Serialize)]
struct Action {
    name: String,
    run: bool,
    reason: &'static str,
    before: Option<String>,
    after: String,
    reuse: Reuse,
}

#[derive(Serialize)]
struct Reuse {
    #[serde(rename = "type")]
    kind: &'static str,
    source: &'static str,
}

fn world(entries: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut world = BTreeMap::new();
    for entry in entries {
        let (name, value) = entry
            .split_once('=')
            .ok_or_else(|| format!("world entry {entry:?} must be NAME=VALUE"))?;
        if name.trim().is_empty() || value.trim().is_empty() {
            return Err(format!("world entry {entry:?} must name a non-empty value"));
        }
        if world.insert(name.to_string(), value.to_string()).is_some() {
            return Err(format!("world entry {name:?} is declared more than once"));
        }
    }
    Ok(world)
}

fn digest(world: &BTreeMap<String, String>, actions: &[Action]) -> String {
    let mut sponge = Sha256::new();
    for (name, value) in world {
        sponge.update(name.as_bytes());
        sponge.update([0]);
        sponge.update(value.as_bytes());
        sponge.update([0]);
    }
    sponge.update([1]);
    for action in actions {
        sponge.update(action.name.as_bytes());
        sponge.update([0]);
        sponge.update(action.after.as_bytes());
        sponge.update([0]);
    }
    format!("{:x}", sponge.finalize())
}
