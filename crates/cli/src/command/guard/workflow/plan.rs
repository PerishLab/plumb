use super::{reuse, tree};
use crate::shape;
use clap::Args;
use plumb::cli::Root;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct Input {
    #[arg(skip)]
    pub(in crate::command) evidence: bool,
    #[arg(long)]
    pub(in crate::command) base: Option<String>,
    #[arg(long = "world")]
    pub(in crate::command) world: Vec<String>,
    #[arg(long = "workload")]
    pub(in crate::command) workload: Vec<String>,
    #[arg(long = "identity")]
    pub(in crate::command) identity: Vec<String>,
    #[arg(long = "project")]
    pub(in crate::command) project: Vec<String>,
    #[arg(long = "root")]
    pub(in crate::command) roots: Vec<String>,
    #[arg(long)]
    pub(in crate::command) inventory: Option<PathBuf>,
    #[arg(long = "inventory-url", value_name = "INVENTORY_URL")]
    pub(in crate::command) source: Option<String>,
    #[command(flatten)]
    pub(in crate::command) target: Root,
}

pub(in crate::command) fn derive(input: Input, wanted: Option<&str>) -> Result<String, String> {
    render(Path::new(&input.target.root), &input, wanted, None)
}

pub(in crate::command) fn production(
    mut input: Input,
    wanted: &str,
    contract: &plumb::rule::Production,
) -> Result<String, String> {
    input
        .world
        .push(format!("production={}", contract.digest()?));
    render(
        Path::new(&input.target.root),
        &input,
        Some(wanted),
        Some(contract),
    )
}

pub fn run(input: Input) -> i32 {
    let root = Path::new(&input.target.root);
    match render(root, &input, None, None) {
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

fn render(
    root: &Path,
    input: &Input,
    wanted: Option<&str>,
    contract: Option<&plumb::rule::Production>,
) -> Result<String, String> {
    let base = input.base.as_deref();
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
    let world = fields("world", &input.world)?;
    let workload = fields("workload", &input.workload)?;
    let publication = fields("identity", &input.identity)?;
    let projects = tree::Projects::parse(&input.project)?;
    let roots = roots(&input.roots)?;
    let inventory = reuse::Inventory::read(input.inventory.as_deref())?
        .at(input.source.as_deref())?
        .evidence(input.evidence);
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
    projects.declare(&mut current.keys, &roots)?;
    if base.is_some() {
        projects.declare(&mut prior.keys, &roots)?;
    }
    if let Some(wanted) = wanted {
        current.keys.retain(|key| key.name() == wanted);
        prior.keys.retain(|key| key.name() == wanted);
    }
    if current.keys.is_empty() {
        return Err(match wanted {
            Some(wanted) => format!("the repository implies no workflow action called {wanted}"),
            None => "the repository implies no workflow action".to_string(),
        });
    }
    let base = base.map(|base| git.revision(base)).transpose()?;
    let head = git.revision("HEAD")?;
    let context = reuse::Context::new(&workload, &world, &publication);
    let actions: Vec<Action> = current
        .keys
        .iter()
        .map(|key| -> Result<Action, String> {
            let prior = prior.keys.iter().find(|held| held.name() == key.name());
            let project = projects.action(&key.name());
            let prior = prior
                .map(|held| before.projected(held, &project))
                .transpose()?;
            let current = after.projected(key, &project)?;
            let moved = prior.as_ref() != Some(&current);
            let keys = context.keys(&key.name(), &current);
            let (verdict, receipt) = inventory.verified(&key.name(), &keys, contract)?;
            let cold = base.is_none() || moved;
            let run = cold && verdict.decision == "run";
            Ok(Action {
                name: key.name(),
                run,
                decision: if cold { verdict.decision } else { "skip" },
                reason: match (&prior, moved, cold, verdict.reason) {
                    (_, _, true, reason) if reason != "record-absent" => reason,
                    (None, _, _, _) => "action-added",
                    (Some(_), true, _, _) => "input-moved",
                    (Some(_), false, _, _) => "input-held",
                },
                before: prior,
                after: current,
                project,
                keys,
                reuse: if cold {
                    verdict.source
                } else {
                    reuse::Source::none()
                },
                depot: if cold { verdict.depot } else { None },
                receipt: if cold { receipt } else { None },
                production: contract.map(plumb::rule::Production::digest).transpose()?,
            })
        })
        .collect::<Result<_, _>>()?;
    let identity = digest(&world, &publication, &actions);
    serde_json::to_string(&Plan {
        schema: "plumb.workflow-plan/v1",
        base,
        head,
        identity,
        publication,
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
    publication: BTreeMap<String, String>,
    actions: Vec<Action>,
}

#[derive(Serialize)]
struct Action {
    name: String,
    run: bool,
    decision: &'static str,
    reason: &'static str,
    before: Option<String>,
    after: String,
    project: Vec<tree::Project>,
    keys: reuse::Keys,
    reuse: reuse::Source,
    #[serde(skip_serializing_if = "Option::is_none")]
    depot: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    receipt: Option<plumb::rule::Receipt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    production: Option<String>,
}

fn fields(kind: &str, entries: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut world = BTreeMap::new();
    for entry in entries {
        let (name, value) = entry
            .split_once('=')
            .ok_or_else(|| format!("{kind} entry {entry:?} must be NAME=VALUE"))?;
        if name.trim().is_empty() || value.trim().is_empty() {
            return Err(format!(
                "{kind} entry {entry:?} must name a non-empty value"
            ));
        }
        if world.insert(name.to_string(), value.to_string()).is_some() {
            return Err(format!("{kind} entry {name:?} is declared more than once"));
        }
    }
    Ok(world)
}

fn roots(entries: &[String]) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut roots: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in entries {
        let (action, path) = entry
            .split_once('=')
            .ok_or_else(|| format!("root entry {entry:?} must be ACTION=PATH"))?;
        if action.trim().is_empty() || !tree::relative(path) {
            return Err(format!(
                "root entry {entry:?} must name an action and relative path"
            ));
        }
        let held = roots.entry(action.to_string()).or_default();
        if held.iter().any(|root| root == path) {
            return Err(format!("root entry {entry:?} is declared more than once"));
        }
        held.push(path.to_string());
        held.sort();
    }
    Ok(roots)
}

fn digest(
    world: &BTreeMap<String, String>,
    identity: &BTreeMap<String, String>,
    actions: &[Action],
) -> String {
    let mut sponge = Sha256::new();
    for (name, value) in world {
        sponge.update(name.as_bytes());
        sponge.update([0]);
        sponge.update(value.as_bytes());
        sponge.update([0]);
    }
    sponge.update([1]);
    for (name, value) in identity {
        sponge.update(name.as_bytes());
        sponge.update([0]);
        sponge.update(value.as_bytes());
        sponge.update([0]);
    }
    sponge.update([2]);
    for action in actions {
        sponge.update(action.name.as_bytes());
        sponge.update([0]);
        sponge.update(action.after.as_bytes());
        sponge.update([0]);
    }
    format!("{:x}", sponge.finalize())
}
