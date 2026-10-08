use super::super::workflow::tree::Tree;
use super::tree::{self, Index};
use plumb::guard::{Action, Descriptor};
use std::path::Path;

struct Check {
    proof: Action,
    commands: Vec<Vec<String>>,
    execution: Option<plumb::config::Execution>,
}

struct Preparation {
    name: String,
    input: String,
    commands: Vec<Vec<String>>,
    probes: Vec<plumb::rule::Probe>,
    tools: Vec<String>,
    environment: Option<plumb::config::Environment>,
    manifest: Option<String>,
    paths: Vec<String>,
}

struct Catalog<'a> {
    root: &'a Path,
    tree: &'a Tree,
    product: &'a str,
}

pub(super) struct Binding<'a> {
    pub execution: Option<&'a plumb::config::Execution>,
    pub probes: &'a std::collections::BTreeMap<String, Vec<plumb::rule::Probe>>,
}

pub(super) fn prove(root: &Path) -> Result<Descriptor, String> {
    prepare(root, false)
}

pub(super) fn resolved(root: &Path) -> Result<Descriptor, String> {
    prepare(root, true)
}

fn prepare(root: &Path, latest: bool) -> Result<Descriptor, String> {
    let tree = plumb::guard::tree(root)?;
    let index = Index::new(root, &tree)?;
    let resolution = Some(if latest {
        crate::command::packages::resolve(&index.root, "ci-latest")?
    } else {
        plumb::packages::locked(&index.root, "local-locked")?
    });
    let captured = Tree::read(&index.root, None)?;
    let manifest = captured.text("plumb.toml")?;
    let product = crate::shape::product::named(manifest.as_deref())?;
    let prepared = Catalog {
        root,
        tree: &captured,
        product: &product,
    }
    .checks()?;
    let mut checks = Vec::new();
    for Preparation {
        name,
        input,
        commands,
        probes: declared,
        tools,
        environment,
        manifest,
        paths,
    } in prepared
    {
        let environment = super::world::environment(&commands, environment)?;
        let mut probes = manifest
            .as_deref()
            .map(|manifest| crate::catalog::probe::read(manifest, paths.iter().map(String::as_str)))
            .transpose()?
            .unwrap_or_default();
        for (at, probe) in declared.into_iter().enumerate() {
            probes.insert(format!("workflow.probe.{name}.{at}"), vec![probe]);
        }
        let mut programs = crate::catalog::probe::programs(&probes)?;
        programs.extend(tools);
        programs.extend(
            commands
                .iter()
                .filter_map(|command| command.first().cloned()),
        );
        if programs.iter().any(|program| program == "cargo") {
            programs.push("rustc".into());
        }
        let execution = Some(super::world::execution(
            &name,
            environment,
            &index.root,
            &programs,
        )?);
        let binding = Binding {
            execution: execution.as_ref(),
            probes: &probes,
        };
        let world = super::world::digest(&name, &input, &commands, &binding)?;
        checks.push(Check {
            proof: Action { name, input, world },
            commands,
            execution,
        });
    }
    if let Ok(proof) = plumb::guard::staged(root, &tree)
        && proof
            .actions
            .iter()
            .eq(checks.iter().map(|check| &check.proof))
        && proof.resolution == resolution
    {
        return Ok(proof);
    }
    let pending = checks
        .iter()
        .filter(|check| !super::cache::contains(&check.proof))
        .collect::<Vec<_>>();
    if !pending.is_empty() {
        for check in pending {
            eprintln!("guard {}", check.proof.name);
            for command in &check.commands {
                tree::execute(
                    &index.root,
                    command,
                    &tree::Execution {
                        execution: check.execution.as_ref(),
                    },
                )?;
                tree::unchanged(&index)?;
            }
            super::cache::record(&check.proof)?;
        }
    }
    let proof = Descriptor::new(
        root,
        tree,
        checks.into_iter().map(|check| check.proof).collect(),
    )?;
    let proof = match resolution {
        Some(resolution) => proof.resolved(resolution)?,
        None => proof,
    };
    plumb::guard::stage(root, &proof)?;
    Ok(proof)
}

impl Catalog<'_> {
    fn checks(&self) -> Result<Vec<Preparation>, String> {
        let tree = self.tree;
        let manifest = tree.text("plumb.toml")?;
        let mut held = manifest
            .as_deref()
            .map(crate::shape::workflow::parse)
            .unwrap_or_default();
        if let Some(error) = &held.refusal {
            return Err(error.clone());
        }
        if held.keys.is_empty() {
            held.keys = crate::shape::workflow::inferred(
                tree.has("Cargo.toml"),
                tree.has("pnpm-lock.yaml"),
                tree.has("plumb.toml"),
                tree.has("ectropy.toml"),
            )
            .keys;
        }
        held.validate()?;
        let mut checks = Vec::new();
        for key in held.keys.iter().filter(|key| key.lane() == "guard") {
            let name = key.name();
            let commands = self.commands(tree, self.product, &name)?;
            if commands.is_empty() {
                continue;
            }
            let input = tree.digest(key);
            let environment = commands
                .iter()
                .any(|command| command.first().is_some_and(|program| program == "cargo"))
                .then(|| super::environment::cargo(self.root))
                .transpose()?;
            checks.push(Preparation {
                probes: held.probes(&name)?,
                tools: held.tools(&name)?,
                name,
                input,
                commands,
                environment,
                manifest: manifest.clone(),
                paths: tree
                    .selection(&key.paths)
                    .iter()
                    .map(|(path, _)| (*path).clone())
                    .collect(),
            });
        }
        if checks.is_empty() {
            return Err("staged tree implies no guard action".into());
        }
        Ok(checks)
    }

    fn commands(&self, tree: &Tree, product: &str, name: &str) -> Result<Vec<Vec<String>>, String> {
        let cargo = |args: &[&str]| {
            std::iter::once("cargo".to_string())
                .chain(args.iter().map(|arg| arg.to_string()))
                .collect()
        };
        Ok(match name {
            "guard/rust" => vec![
                cargo(&["fmt", "--all", "--check"]),
                cargo(&[
                    "clippy",
                    "--locked",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ]),
                cargo(&[
                    "check",
                    "--locked",
                    "--workspace",
                    "--all-targets",
                    "--release",
                ]),
            ],
            "guard/test" => vec![cargo(&["test", "--locked"])],
            "guard/web" => super::web::commands(tree)?,
            "guard/plumb" if product == "plumb" => vec![cargo(&[
                "run", "--quiet", "--locked", "--bin", "plumb", "--", "doctor", ".",
            ])],
            "guard/plumb" => vec![vec!["plumb".into(), "doctor".into(), ".".into()]],
            "guard/ectropy" if product == "ectropy" => vec![cargo(&[
                "run", "--quiet", "--locked", "--bin", "ectropy", "--", ".",
            ])],
            "guard/ectropy" => vec![vec!["ectropy".into(), ".".into()]],
            _ => Vec::new(),
        })
    }
}
