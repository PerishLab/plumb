use super::super::workflow::tree::Tree;
pub(super) use super::super::world::{digest, execution};
use super::tree::Index;
use crate::command::cache::transport::Context;
use std::path::Path;

pub(super) struct Check {
    pub proof: plumb::guard::Action,
    pub commands: Vec<Vec<String>>,
    pub exclusions: Vec<String>,
    pub execution: Option<plumb::config::Execution>,
}

pub(super) struct Prepared {
    pub checks: Vec<Check>,
    pub resolution: Option<plumb::packages::Resolution>,
    pub material: String,
    pub context: Option<Context>,
}

impl Prepared {
    pub fn new(
        root: &Path,
        index: &Index,
        latest: bool,
        seat: Option<&Path>,
    ) -> Result<Self, String> {
        let (locked, observed) = if latest {
            let (locked, observed) = crate::diagnostic::measure("packages.resolve", || {
                crate::command::packages::observed(&index.root, "ci-latest")
            })?;
            (locked, Some(plumb::packages::Observation::new(observed)))
        } else {
            (plumb::packages::locked(&index.root, "local-locked")?, None)
        };
        index.settle()?;
        let resolution = Some(locked);
        let captured = Tree::read(&index.root, None)?;
        let manifest = captured.text("plumb.toml")?;
        let product = crate::shape::product::named(manifest.as_deref())?;
        let prepared = super::action::Catalog {
            tree: &captured,
            product: &product,
        }
        .checks()?;
        let observation = if prepared.iter().any(|check| check.name == "guard/plumb") {
            Some(match observed {
                Some(observed) => observed,
                None => plumb::packages::Observation::npm(&index.root, "local-locked")?,
            })
        } else {
            None
        };
        let context = Context::read(&index.root)?;
        let mut materials = Vec::new();
        let inputs = std::env::vars_os().collect::<Vec<_>>();
        let mut checks = Vec::new();
        for super::action::Preparation {
            name,
            input,
            mut commands,
            probes: declared,
            tools,
            manifest,
            paths,
        } in prepared
        {
            let (exclusions, policy) = if name == "guard/web" {
                let policy = plumb::packages::Policy::read(&index.root)?;
                let mut flags = policy.flags.clone();
                let home = plumb::config::value("PLUMB_HOME")
                    .map(std::path::PathBuf::from)
                    .or_else(|| plumb::config::data("plumb"))
                    .ok_or("cannot own the Guard pnpm cache: no PLUMB_HOME")?;
                flags.push(plumb::packages::cache(&home)?);
                flags.push("--config.verify-deps-before-run=error".into());
                for command in commands
                    .iter_mut()
                    .filter(|command| command.first().is_some_and(|program| program == "pnpm"))
                {
                    command.splice(1..1, flags.iter().cloned());
                }
                (flags, policy)
            } else {
                Default::default()
            };
            let mut probes = manifest
                .as_deref()
                .map(|manifest| {
                    crate::catalog::probe::read(manifest, paths.iter().map(String::as_str))
                })
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
            if seat.is_some() {
                programs.push("cargo".into());
            }
            if programs.iter().any(|program| program == "cargo") {
                programs.push("rustc".into());
            }
            let environment = super::environment::capture(root, &programs, &inputs)?;
            let environment = match name.as_str() {
                "guard/plumb" => environment,
                _ => environment.combine(policy.environment()?)?,
            };
            let environment = match &context {
                Some(context) => context.bind(environment)?,
                None => environment,
            };
            let environment = match seat {
                Some(seat) => super::environment::managed(environment, seat)?,
                None => environment,
            };
            let mut execution =
                super::world::execution(&name, environment, &index.root, &programs)?;
            let material = if name == "guard/plumb" {
                let binding = super::action::Binding {
                    execution: Some(&execution),
                    probes: &probes,
                };
                let material = super::world::digest(&name, &input, &commands, &binding)?.material;
                execution.environment =
                    super::environment::observed(execution.environment, observation.as_ref())?;
                Some(material)
            } else {
                None
            };
            let execution = Some(execution);
            let binding = super::action::Binding {
                execution: execution.as_ref(),
                probes: &probes,
            };
            let world = super::world::digest(&name, &input, &commands, &binding)?;
            materials.push(material.unwrap_or(world.material));
            checks.push(Check {
                proof: plumb::guard::Action {
                    name,
                    input,
                    world: world.proof,
                },
                commands,
                exclusions,
                execution,
            });
        }
        let material = super::super::world::material(
            &materials,
            &resolution
                .as_ref()
                .expect("resolution was captured")
                .packages,
        )?;
        Ok(Self {
            checks,
            resolution,
            material,
            context,
        })
    }

    pub fn reused(&self, root: &Path, tree: &str) -> Option<plumb::guard::Descriptor> {
        plumb::guard::staged(root, tree).ok().filter(|proof| {
            proof
                .actions
                .iter()
                .eq(self.checks.iter().map(|check| &check.proof))
                && proof.resolution == self.resolution
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(root: &Path, args: &[&str]) {
        let output = plumb::config::detached("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    struct Frame {
        material: String,
        proof: String,
        observation: String,
    }

    fn prepare(root: &Path) -> Frame {
        let tree = plumb::guard::tree(root).unwrap();
        let index = Index::new(root, &tree).unwrap();
        let seat = super::super::cargo::seat(root, &tree).unwrap();
        let prepared = Prepared::new(root, &index, false, seat.as_deref()).unwrap();
        let check = prepared
            .checks
            .iter()
            .find(|check| check.proof.name == "guard/plumb")
            .unwrap();
        let text = check
            .execution
            .as_ref()
            .unwrap()
            .environment
            .get("PLUMB_PACKAGES_OBSERVATION")
            .unwrap();
        let observation = plumb::packages::Observation::read(&index.root, text).unwrap();
        assert_eq!(observation.resolution.tree, tree);
        Frame {
            material: prepared.material,
            proof: check.proof.world.clone(),
            observation: text.to_owned(),
        }
    }

    #[test]
    fn source() {
        if std::env::var_os("PLUMB_WORLD_TEST").is_none() {
            let home = tempfile::tempdir().unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "command::guard::precommit::world::tests::source",
                    "--nocapture",
                ])
                .env("PLUMB_WORLD_TEST", "child")
                .env("PLUMB_HOME", home.path())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        plumb::depot::carry(crate::catalog::carried::FILES);
        let root = tempfile::tempdir().unwrap();
        git(root.path(), &["init", "-q"]);
        std::fs::create_dir(root.path().join("src")).unwrap();
        std::fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='probe'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        std::fs::write(root.path().join("plumb.toml"), "[identity]\nns='probe'\n[workflow.hash.guard]\nrust=['Cargo.toml','Cargo.lock','src']\nplumb=['Cargo.toml','Cargo.lock','src']\n").unwrap();
        std::fs::write(
            root.path().join("src/lib.rs"),
            "pub fn answer() -> u8 { 1 }\n",
        )
        .unwrap();
        let output = std::process::Command::new("cargo")
            .args(["generate-lockfile", "--offline"])
            .current_dir(root.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        git(root.path(), &["add", "."]);
        let first = prepare(root.path());
        std::fs::write(
            root.path().join("src/lib.rs"),
            "pub fn answer() -> u8 { 2 }\n",
        )
        .unwrap();
        git(root.path(), &["add", "src/lib.rs"]);
        let next = prepare(root.path());
        assert_ne!(
            first.observation, next.observation,
            "current observations must bind distinct source trees"
        );
        assert_ne!(first.proof, next.proof, "current Guard proof must change");
        assert!(plumb::packages::Observation::read(root.path(), &first.observation).is_err());
        assert!(plumb::packages::Observation::read(root.path(), &next.observation).is_ok());
        assert_eq!(
            first.material, next.material,
            "source-only Doctor tree changes must preserve compilation identity"
        );
    }
}
