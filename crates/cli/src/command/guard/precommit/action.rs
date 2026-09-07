use plumb::guard::{Action, Descriptor};
use std::path::{Path, PathBuf};

use super::super::workflow::tree::Tree;
use super::tree::{self, Index};

struct Check {
    proof: Action,
    commands: Vec<Vec<String>>,
    environment: Option<plumb::config::Environment>,
}

struct Catalog<'a> {
    root: &'a Path,
    product: &'a crate::shape::product::Target,
    binding: Binding<'a>,
}

pub(super) struct Binding<'a> {
    pub configuration: Option<&'a str>,
    pub profile: Option<&'a crate::shape::product::Profile>,
    pub environment: Option<&'a plumb::config::Environment>,
}

pub(super) fn prove(root: &Path) -> Result<Descriptor, String> {
    if plumb::config::value("PLUMB_HOME").is_none()
        && (plumb::config::value("PLUMB_GUARD_CONFIGURATION").is_some()
            || plumb::config::value("PLUMB_GUARD_DEPOT").is_some())
    {
        return Err("guard depot binding is internal to one isolated guard action".into());
    }
    let tree = tree::git(root, &["write-tree"], "read staged tree")?;
    let target = super::configuration::target(root)?;
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
    let product = crate::shape::product::guard(root, "")?;
    let mut index = mismatched
        .then(|| isolate(root, &tree, product.profile.as_ref()))
        .transpose()?;
    let configuration = index
        .as_ref()
        .map(|index| {
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
    let checks = Catalog {
        root,
        product: &product,
        binding: Binding {
            configuration: configuration.as_ref().map(|held| held.mark()),
            profile: product.profile.as_ref(),
            environment: None,
        },
    }
    .checks()?;
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
        if index.is_none() {
            index = Some(isolate(root, &tree, product.profile.as_ref())?);
        }
        let index = index.as_ref().expect("a pending guard has an index");
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
                        environment: check.environment.as_ref(),
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
    fn checks(&self) -> Result<Vec<Check>, String> {
        let tree = Tree::read(self.root, None)?;
        let governed = self.product.profile.is_some();
        profile(&tree, self.product.profile.as_ref())?;
        let mut held = crate::shape::workflow::read(self.root);
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
            let commands = self.commands(&self.product.product, &name)?;
            if commands.is_empty() {
                continue;
            }
            let input = tree.digest(key);
            let environment = commands
                .iter()
                .any(|command| command.first().is_some_and(|program| program == "cargo"))
                .then(|| super::environment::cargo(self.root))
                .transpose()?;
            let binding = Binding {
                environment: environment.as_ref(),
                ..self.binding
            };
            let world = super::world::digest(&name, &input, &commands, &binding)?;
            checks.push(Check {
                proof: Action { name, input, world },
                commands,
                environment,
            });
        }
        if checks.is_empty() {
            return Err("staged tree implies no guard action".into());
        }
        Ok(checks)
    }

    fn commands(&self, product: &str, name: &str) -> Result<Vec<Vec<String>>, String> {
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
            "guard/web" => self.web()?,
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

    fn web(&self) -> Result<Vec<Vec<String>>, String> {
        let mut held = vec![
            vec!["corepack".into(), "enable".into()],
            vec!["pnpm".into(), "install".into(), "--frozen-lockfile".into()],
        ];
        if self.root.join("biome.json").is_file() {
            held.push(vec!["pnpm".into(), "biome".into(), "ci".into(), ".".into()]);
        }
        held.push(vec![
            "pnpm".into(),
            "-r".into(),
            "exec".into(),
            "tsc".into(),
            "--noEmit".into(),
        ]);
        held.push(vec!["pnpm".into(), "-r".into(), "test".into()]);
        let Ok(entries) = std::fs::read_dir(self.root.join("apps")) else {
            return Ok(held);
        };
        let mut sites = Vec::new();
        for entry in entries.flatten() {
            let Ok(text) = std::fs::read_to_string(entry.path().join("package.json")) else {
                continue;
            };
            let Ok(doc) = serde_json::from_str::<serde_json::Value>(&text) else {
                continue;
            };
            if doc.pointer("/scripts/build").is_some()
                && let Some(name) = doc.get("name").and_then(serde_json::Value::as_str)
            {
                sites.push(name.to_string());
            }
        }
        sites.sort();
        for site in sites {
            held.push(vec!["pnpm".into(), "--filter".into(), site, "build".into()]);
        }
        Ok(held)
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

fn isolate(
    root: &Path,
    tree: &str,
    profile: Option<&crate::shape::product::Profile>,
) -> Result<Index, String> {
    let index = Index::new(root, tree)?;
    if let Some(profile) = profile {
        index.govern(profile)?;
    }
    Ok(index)
}
