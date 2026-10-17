use super::super::workflow::tree::Tree;
use super::tree::{self, Index};
use super::world::Prepared;
use plumb::guard::Descriptor;
use std::path::Path;

pub(super) struct Preparation {
    pub name: String,
    pub input: String,
    pub commands: Vec<Vec<String>>,
    pub probes: Vec<plumb::rule::Probe>,
    pub tools: Vec<String>,
    pub manifest: Option<String>,
    pub paths: Vec<String>,
}

pub(super) struct Catalog<'a> {
    pub tree: &'a Tree,
    pub product: &'a str,
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

#[locus::trace(with = crate::observation::view())]
fn prepare(root: &Path, latest: bool) -> Result<Descriptor, String> {
    let tree = plumb::guard::tree(root)?;
    let seat = super::cargo::seat(root, &tree)?;
    if plumb::guard::staged(root, &tree).is_ok() {
        let index = Index::new(root, &tree)?;
        let prepared = Prepared::new(root, &index, latest, seat.as_deref())?;
        if let Some(proof) = prepared.reused(root, &tree) {
            return Ok(proof);
        }
    }
    let lease = seat
        .is_some()
        .then(|| super::cargo::Lease::new(root))
        .transpose()?;
    let index = Index::new(root, &tree)?;
    let prepared = Prepared::new(root, &index, latest, seat.as_deref())?;
    if let Some(proof) = prepared.reused(root, &tree) {
        return Ok(proof);
    }
    let Prepared { checks, resolution } = prepared;
    let pending = checks
        .iter()
        .filter(|check| !super::cache::contains(&check.proof))
        .collect::<Vec<_>>();
    if !pending.is_empty() {
        if let Some(check) = pending.iter().find(|check| check.proof.name == "guard/web") {
            plumb::packages::fetch(&index.root, &check.exclusions)?;
            tree::unchanged(&index)?;
        }
        for check in pending {
            eprintln!("guard {}", check.proof.name);
            let execution = tree::Execution {
                execution: check.execution.as_ref(),
                observation: crate::observation::bind("check", &check.proof.name),
            };
            for command in &check.commands {
                tree::execute(&index.root, command, &execution)?;
                tree::unchanged(&index)?;
            }
            if let Some(lease) = &lease {
                lease.settle();
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
    pub(super) fn checks(&self) -> Result<Vec<Preparation>, String> {
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
            checks.push(Preparation {
                probes: held.probes(&name)?,
                tools: held.tools(&name)?,
                name,
                input,
                commands,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned() {
        let tree = Tree::default();
        let catalog = Catalog {
            tree: &tree,
            product: "product",
        };
        assert_eq!(
            catalog.commands(&tree, "product", "guard/test").unwrap(),
            vec![vec!["cargo".to_string(), "test".into(), "--locked".into()]],
            "Wharf skips its release cargo suite because guard/test runs exactly this on a virtual workspace (PerishLab/wharf#127); change both together"
        );
    }
}
