use super::super::workflow::tree::Tree;
pub(super) use super::super::world::{digest, execution};
use super::Selection;
use super::tree::Index;
use crate::command::cache::transport::Context;
use std::path::Path;

pub(super) struct Check {
    pub proof: plumb::guard::Action,
    pub commands: Vec<Vec<String>>,
    pub exclusions: Vec<String>,
    pub execution: Option<std::sync::Arc<plumb::config::Execution>>,
    pub material: String,
    pub protection: Option<crate::execution::lock::Protection>,
    pub brand: Option<crate::consumption::brand::Prepared>,
}

pub(super) struct Prepared {
    pub checks: Vec<Check>,
    pub resolution: Option<plumb::packages::Resolution>,
    pub depot: String,
    pub material: String,
    pub context: Option<Context>,
}

impl Prepared {
    pub fn new(
        root: &Path,
        index: &Index,
        selection: Selection,
        seat: Option<&Path>,
    ) -> Result<Self, String> {
        let rules = plumb::depot::rules()?;
        let depot = rules.mark().to_string();
        let (locked, observed) = if matches!(selection, Selection::Latest) {
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
        let topology = crate::shape::topology::world(&index.root, manifest.as_deref())?;
        index.settle()?;
        let prepared = selection.checks(&captured, &product)?;
        let protection = crate::execution::lock::Protection::read(
            &index.root,
            !matches!(selection, Selection::Compiler),
            prepared.iter().map(|check| check.name.as_str()),
            resolution.clone(),
        )?;
        let observation = prepared
            .iter()
            .any(|check| check.name == "guard/plumb")
            .then(|| match observed {
                Some(observed) => Ok(observed),
                None => plumb::packages::Observation::npm(&index.root, "local-locked"),
            })
            .transpose()?;
        let context = Context::read(&index.root)?;
        let inputs = std::env::vars_os().collect::<Vec<_>>();
        let mut checks = Vec::new();
        for mut item in prepared {
            let input = crate::shape::topology::bind(&item.name, item.input, topology.as_deref());
            let (input, brand) = crate::consumption::brand::prepare(
                (&item.name, input),
                &index.root,
                resolution.as_ref(),
            )?;
            let (exclusions, policy) = if item.name == "guard/web" {
                super::web::prepare(&index.root, &mut item.commands)?
            } else {
                Default::default()
            };
            let mut probes = item
                .manifest
                .as_deref()
                .map(|m| crate::catalog::probe::read(m, item.paths.iter().map(String::as_str)))
                .transpose()?
                .unwrap_or_default();
            for (at, probe) in item.probes.into_iter().enumerate() {
                probes.insert(format!("workflow.probe.{}.{at}", item.name), vec![probe]);
            }
            let mut programs = crate::catalog::probe::programs(&probes)?;
            programs.extend(item.tools);
            programs.extend(item.commands.iter().filter_map(|c| c.first().cloned()));
            if seat.is_some() {
                programs.push("cargo".into());
            }
            if programs.iter().any(|program| program == "cargo") {
                programs.push("rustc".into());
            }
            let mut env = super::environment::capture(root, &programs, &inputs)?;
            if item.name != "guard/plumb" {
                env = env.combine(policy.environment()?)?;
            }
            if let Some(context) = &context {
                env = context.bind(env)?;
            }
            if let Some(seat) = seat {
                env = super::environment::managed(env, seat)?;
            }
            let mut execution = super::world::execution(&item.name, env, &index.root, &programs)?;
            let input = protection
                .as_ref()
                .map(|h| h.bind(&item.name, &input))
                .unwrap_or(input);
            let mut material = None;
            if item.name == "guard/plumb" {
                let binding = super::action::Binding {
                    execution: Some(&execution),
                    probes: &probes,
                };
                let digested = super::world::digest(&item.name, &input, &item.commands, &binding)?;
                material = Some(digested.material);
                let obs = observation.as_ref();
                execution.environment = super::environment::observed(execution.environment, obs)?;
                if product != "plumb" {
                    execution.environment =
                        super::environment::depot(execution.environment, rules)?;
                }
            }
            let execution = Some(std::sync::Arc::new(execution));
            let binding = super::action::Binding {
                execution: execution.as_deref(),
                probes: &probes,
            };
            let world = super::world::digest(&item.name, &input, &item.commands, &binding)?;
            let proof = plumb::guard::Action {
                name: item.name,
                input,
                world: world.proof,
            };
            let check = Check {
                material: material.unwrap_or(world.material),
                protection: None,
                brand,
                proof,
                commands: item.commands,
                exclusions,
                execution,
            };
            super::cargo::register(&mut checks, protection.as_ref(), &depot, check)?;
        }
        let materials: Vec<_> = checks.iter().map(|c| c.material.clone()).collect();
        let material = super::super::world::material(&materials)?;
        Ok(Self {
            checks,
            resolution,
            depot,
            material,
            context,
        })
    }

    pub fn reused(&self, root: &Path, tree: &str) -> Option<plumb::guard::Descriptor> {
        plumb::guard::staged(root, tree).ok().filter(|proof| {
            let actions = self.checks.iter().map(|check| &check.proof);
            proof.depot == self.depot
                && proof.actions.iter().eq(actions)
                && proof.resolution == self.resolution
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plumb::guard::Descriptor;
    use plumb::packages::Observation;
    use std::{env, fs, process::Command};

    #[derive(serde::Deserialize, serde::Serialize)]
    struct Frame {
        depot: String,
        material: String,
        proof: String,
        rust: String,
        observation: String,
    }

    fn prepare(root: &Path, stage: bool, hosted: bool) -> Frame {
        let tree = plumb::guard::tree(root).unwrap();
        let index = Index::new(root, &tree).unwrap();
        let seat = super::super::cargo::seat(root, &tree).unwrap();
        let mut prepared = Prepared::new(root, &index, Selection::Local, seat.as_deref()).unwrap();
        let checks = &prepared.checks;
        let find = |name| checks.iter().find(|c| c.proof.name == name).unwrap();
        let (plumb, rust) = (find("guard/plumb"), find("guard/rust"));
        let env = &plumb.execution.as_ref().unwrap().environment;
        let other = &rust.execution.as_ref().unwrap().environment;
        if hosted {
            assert!(
                env.get("PLUMB_DEPOT_MARK").is_none() && env.get("PLUMB_CONFIGURATION").is_none()
            );
        } else {
            assert_eq!(env.get("PLUMB_DEPOT_MARK"), Some(prepared.depot.as_str()));
        }
        assert!(
            other.get("PLUMB_DEPOT_MARK").is_none() && other.get("PLUMB_CONFIGURATION").is_none()
        );
        let text = env.get("PLUMB_PACKAGES_OBSERVATION").unwrap();
        let seen = Observation::read(&index.root, text).unwrap();
        assert_eq!(seen.resolution.tree, tree);
        let actions = checks.iter().map(|c| c.proof.clone()).collect();
        let proof = Descriptor::new(root, tree.clone(), actions).unwrap();
        let resolution = prepared.resolution.clone().unwrap();
        let proof = proof.resolved(resolution).unwrap();
        assert_eq!(proof.depot, prepared.depot);
        let frame = Frame {
            depot: proof.depot.clone(),
            material: prepared.material.clone(),
            proof: plumb.proof.world.clone(),
            rust: rust.proof.world.clone(),
            observation: text.to_owned(),
        };
        if stage {
            plumb::guard::stage(root, &proof).unwrap();
            assert!(prepared.reused(root, &tree).is_some());
            prepared.depot = "0".repeat(64);
        }
        assert!(prepared.reused(root, &tree).is_none());
        frame
    }

    fn child(home: &Path, root: &Path, mode: &str) -> Frame {
        let test = "command::guard::precommit::world::tests::source";
        let result = Command::new(env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env("PLUMB_WORLD_TEST", mode)
            .env("PLUMB_WORLD_ROOT", root)
            .env("PLUMB_HOME", home)
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&result.stderr);
        assert!(result.status.success(), "{err}");
        serde_json::from_slice(&fs::read(home.join(format!("{mode}.json"))).unwrap()).unwrap()
    }

    #[test]
    fn source() {
        if let Some(mode) = env::var_os("PLUMB_WORLD_TEST") {
            plumb::depot::carry(crate::catalog::carried::FILES);
            let root = env::var_os("PLUMB_WORLD_ROOT").unwrap();
            let (root, mode) = (Path::new(&root), mode.to_string_lossy());
            let frame = if mode == "source" {
                let first = prepare(root, true, false);
                fs::write(root.join("src/lib.rs"), "pub fn answer() -> u8 { 2 }\n").unwrap();
                super::super::tree::git(root, &["add", "src/lib.rs"], "world").unwrap();
                let next = prepare(root, true, false);
                assert_ne!(first.observation, next.observation);
                assert_ne!(first.proof, next.proof);
                assert!(Observation::read(root, &first.observation).is_err());
                assert!(Observation::read(root, &next.observation).is_ok());
                assert_eq!(first.material, next.material);
                let saved = fs::read_to_string(root.join("plumb.toml")).unwrap();
                fs::write(root.join("plumb.toml"), saved.replace("probe", "plumb")).unwrap();
                super::super::tree::git(root, &["add", "plumb.toml"], "self").unwrap();
                assert_eq!(prepare(root, false, true).depot, next.depot);
                fs::write(root.join("plumb.toml"), saved).unwrap();
                super::super::tree::git(root, &["add", "plumb.toml"], "restore").unwrap();
                next
            } else {
                prepare(root, false, false)
            };
            let out = Path::new(&env::var_os("PLUMB_HOME").unwrap()).join(format!("{mode}.json"));
            fs::write(out, serde_json::to_vec(&frame).unwrap()).unwrap();
            return;
        }
        let (home, root) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let (home, root) = (home.path(), root.path());
        let git = |a: &[&str]| super::super::tree::git(root, a, "world fixture").unwrap();
        let origin = "https://github.com/Example/probe.git";
        git(&["init", "-q"]);
        git(&["remote", "add", "origin", origin]);
        fs::create_dir(root.join("src")).unwrap();
        let manifest = "[package]\nname='probe'\nversion='0.1.0'\nedition='2024'\n";
        let config = "[identity]\nns='probe'\n[workflow.hash.guard]\nrust=['Cargo.toml','Cargo.lock','src']\nplumb=['Cargo.toml','Cargo.lock','src']\n";
        fs::write(root.join("Cargo.toml"), manifest).unwrap();
        fs::write(root.join("plumb.toml"), config).unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn answer() -> u8 { 1 }\n").unwrap();
        let mut cargo = Command::new("cargo");
        cargo.args(["generate-lockfile", "--offline"]);
        assert!(cargo.current_dir(root).status().unwrap().success());
        git(&["add", "."]);
        let first = child(home, root, "source");
        fs::create_dir_all(home.join("overlay")).unwrap();
        fs::write(home.join("overlay/probe.txt"), "overlay\n").unwrap();
        let next = child(home, root, "overlay");
        assert_ne!(first.depot, next.depot);
        assert_ne!(first.proof, next.proof);
        assert_ne!(first.rust, next.rust);
        assert_eq!(first.material, next.material);
    }
}
