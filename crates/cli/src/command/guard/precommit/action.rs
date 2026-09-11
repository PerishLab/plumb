use super::super::workflow::tree::Tree;
use super::tree::{self, Index};
use plumb::guard::{Action, Descriptor};
use std::path::{Path, PathBuf};

struct Check {
    proof: Action,
    commands: Vec<Vec<String>>,
    execution: Option<plumb::config::Execution>,
}

struct Preparation {
    name: String,
    input: String,
    commands: Vec<Vec<String>>,
    environment: Option<plumb::config::Environment>,
    manifest: Option<String>,
    paths: Vec<String>,
}

struct Catalog<'a> {
    root: &'a Path,
    tree: &'a Tree,
    product: &'a crate::shape::product::Target,
}

pub(super) struct Binding<'a> {
    pub root: &'a Path,
    pub configuration: Option<&'a str>,
    pub profile: Option<&'a crate::shape::product::Profile>,
    pub execution: Option<&'a plumb::config::Execution>,
    pub probes: &'a std::collections::BTreeMap<String, Vec<plumb::rule::Probe>>,
}

pub(super) fn prove(root: &Path) -> Result<Descriptor, String> {
    if plumb::config::value("PLUMB_HOME").is_none()
        && (plumb::config::value("PLUMB_GUARD_CONFIGURATION").is_some()
            || plumb::config::value("PLUMB_GUARD_DEPOT").is_some())
    {
        return Err("guard depot binding is internal to one isolated guard action".into());
    }
    let tree = tree::git(root, &["write-tree"], "read staged tree")?;
    let captured = Tree::read(root, Some(&tree))?;
    let target = super::configuration::target(&captured)?;
    let mismatched = match target.as_deref() {
        Some(target) => {
            let root = plumb::depot::root(&PathBuf::new())?;
            plumb::depot::Rules::at(&root, plumb::version!("PLUMB"))
                .ok()
                .and_then(|held| held.version().map(str::to_string))
                .as_deref()
                != Some(target)
        }
        None => false,
    };
    let manifest = captured.text("plumb.toml")?;
    let index = Index::new(root, &tree)?;
    let configuration = mismatched
        .then(|| {
            super::configuration::Seat::new(
                root,
                &index.root,
                target
                    .as_deref()
                    .expect("a mismatched Plumb version has a target"),
            )
        })
        .transpose()?;
    if let Some(configuration) = &configuration {
        let target = target
            .as_deref()
            .expect("a temporary configuration has a target");
        if configuration.transaction() {
            plumb::depot::guard(configuration.path(), target)?;
        }
        crate::catalog::set::guard(configuration.path(), target)?;
    }
    let product = match &configuration {
        Some(configuration) => configuration.product(root)?,
        None => crate::shape::product::guard(root, manifest.as_deref())?,
    };
    let prepared = Catalog {
        root,
        tree: &captured,
        product: &product,
    }
    .checks()?;
    if let Some(profile) = &product.profile {
        index.govern(profile)?;
    }
    let mut checks = Vec::new();
    for Preparation {
        name,
        input,
        commands,
        environment,
        manifest,
        paths,
    } in prepared
    {
        let environment = super::world::environment(&commands, environment, root, mismatched)?;
        let probes = manifest
            .as_deref()
            .map(|manifest| crate::catalog::probe::read(manifest, paths.iter().map(String::as_str)))
            .transpose()?
            .unwrap_or_default();
        let mut programs = crate::catalog::probe::programs(&probes)?;
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
            root: &index.root,
            configuration: configuration.as_ref().map(|held| held.mark()),
            profile: product.profile.as_ref(),
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
    if !mismatched
        && let Ok(proof) = plumb::guard::staged(root, &tree)
        && proof
            .actions
            .iter()
            .eq(checks.iter().map(|check| &check.proof))
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
                let seat = configuration.as_ref().map(|held| held.path());
                tree::execute(
                    &index.root,
                    command,
                    &tree::Execution {
                        seat,
                        governed: product.profile.is_some(),
                        execution: check.execution.as_ref(),
                    },
                )?;
            }
            super::cache::record(&check.proof)?;
        }
    }
    let proof = Descriptor::new(
        root,
        tree,
        checks.into_iter().map(|check| check.proof).collect(),
    )?;
    plumb::guard::stage(root, &proof)?;
    Ok(proof)
}

impl Catalog<'_> {
    fn checks(&self) -> Result<Vec<Preparation>, String> {
        let tree = self.tree;
        let governed = self.product.profile.is_some();
        profile(tree, self.product.profile.as_ref())?;
        let manifest = match &self.product.profile {
            Some(profile) => Some(profile.manifest.clone()),
            None => tree.text("plumb.toml")?,
        };
        let mut held = manifest
            .as_deref()
            .map(crate::shape::workflow::parse)
            .unwrap_or_default();
        if let Some(error) = held.refusal {
            return Err(error);
        }
        if held.keys.is_empty() {
            held = crate::shape::workflow::inferred(
                tree.has("Cargo.toml"),
                tree.has("pnpm-lock.yaml"),
                tree.has("plumb.toml") || governed,
                tree.has("ectropy.toml") || governed,
            );
        }
        let mut checks = Vec::new();
        for key in held.keys.iter().filter(|key| key.lane() == "guard") {
            let name = key.name();
            let commands = self.commands(tree, &self.product.product, &name)?;
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
                cargo(&["clippy", "--all-targets", "--", "-D", "warnings"]),
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

fn profile(tree: &Tree, profile: Option<&crate::shape::product::Profile>) -> Result<(), String> {
    let Some(profile) = profile else {
        return Ok(());
    };
    match profile.source {
        crate::shape::product::Source::Repository => exact(tree, profile),
        crate::shape::product::Source::Depot => {
            if tree.has("plumb.toml") || tree.has("ectropy.toml") {
                return Err(
                    "a Depot-governed product must not carry plumb.toml or ectropy.toml".into(),
                );
            }
            Ok(())
        }
    }
}

fn exact(tree: &Tree, profile: &crate::shape::product::Profile) -> Result<(), String> {
    for (name, expected) in [
        ("plumb.toml", profile.manifest.as_str()),
        ("ectropy.toml", profile.ectropy.as_str()),
    ] {
        let actual = tree
            .text(name)?
            .ok_or_else(|| format!("{name} is required by its repository migration state"))?;
        if actual != expected {
            return Err(format!("{name} differs from its exact Depot profile"));
        }
    }
    Ok(())
}
