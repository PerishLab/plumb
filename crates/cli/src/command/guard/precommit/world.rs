use super::super::workflow::tree::Tree;
use super::tree::Index;
use crate::command::cache::transport::Context;
use sha2::{Digest as _, Sha256};
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
            let (locked, observed) = crate::command::packages::observed(&index.root, "ci-latest")?;
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
                "guard/plumb" => super::environment::observed(environment, observation.as_ref())?,
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
            let execution = Some(super::world::execution(
                &name,
                environment,
                &index.root,
                &programs,
            )?);
            let binding = super::action::Binding {
                execution: execution.as_ref(),
                probes: &probes,
            };
            let world = super::world::digest(&name, &input, &commands, &binding)?;
            materials.push(world.material);
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
        let material =
            plumb::depot::sha(&serde_json::to_vec(&materials).map_err(|error| error.to_string())?);
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

pub(super) struct Fingerprint {
    material: String,
    proof: String,
}

pub(super) fn digest(
    name: &str,
    input: &str,
    commands: &[Vec<String>],
    binding: &super::action::Binding<'_>,
) -> Result<Fingerprint, String> {
    let mut sponge = Sha256::new();
    sponge.update(name.as_bytes());
    sponge.update([0]);
    sponge.update(serde_json::to_vec(commands).map_err(|error| error.to_string())?);
    sponge.update([0]);
    sponge.update(plumb::version!("PLUMB").as_bytes());
    if let Some(commit) = plumb::commit!("PLUMB") {
        sponge.update([0]);
        sponge.update(commit.as_bytes());
    }
    sponge.update([0]);
    sponge.update(plumb::depot::rules()?.mark().as_bytes());
    sponge.update([0]);
    sponge.update(plumb::config::platform().as_bytes());
    if let Some(execution) = binding.execution {
        sponge.update([0]);
        sponge.update(execution.evidence()?);
        sponge.update([0]);
        sponge.update(crate::catalog::probe::evidence(binding.probes, execution)?);
    }
    for tool in tools(name) {
        if binding.execution.is_some()
            && crate::catalog::probe::covers(binding.probes, &[tool, "--version"])?
        {
            continue;
        }
        sponge.update([0]);
        sponge.update(tool.as_bytes());
        sponge.update([0]);
        sponge.update(version(tool, binding.execution)?.as_bytes());
    }
    let material = format!("{:x}", sponge.clone().finalize());
    sponge.update([0]);
    sponge.update(input.as_bytes());
    Ok(Fingerprint {
        material,
        proof: format!("{:x}", sponge.finalize()),
    })
}

fn tools(name: &str) -> &'static [&'static str] {
    match name {
        "guard/rust" | "guard/test" => &["cargo", "rustc"],
        "guard/web" => &["node", "pnpm"],
        "guard/ectropy" => &["ectropy"],
        "guard/plumb" => &["plumb"],
        _ => &[],
    }
}

pub(super) fn execution(
    name: &str,
    mut environment: plumb::config::Environment,
    root: &std::path::Path,
    selected: &[String],
) -> Result<plumb::config::Execution, String> {
    let mut programs = tools(name)
        .iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    programs.extend(selected.iter().cloned());
    let cargo = programs.iter().any(|program| program == "cargo");
    if cargo {
        crate::execution::cargo::msvc::environment(&mut environment)?;
        programs.extend(
            crate::execution::cargo::msvc::programs()
                .iter()
                .map(|program| (*program).to_string()),
        );
    }
    programs.sort();
    programs.dedup();
    let execution = plumb::config::Execution::new(environment, &programs, root)?;
    if cargo {
        crate::execution::cargo::msvc::tools(&execution)?;
    }
    Ok(execution)
}

fn version(tool: &str, execution: Option<&plumb::config::Execution>) -> Result<String, String> {
    let output = match execution {
        Some(execution) => execution.output(&[tool.into(), "--version".into()]),
        None => plumb::config::detached(tool)
            .arg("--version")
            .output()
            .map_err(|error| error.to_string()),
    }
    .map_err(|error| format!("cannot run {tool} --version: {error}"))?;
    if output.status.success() {
        let stdout = std::str::from_utf8(&output.stdout)
            .map_err(|_| format!("{tool} --version stdout is not UTF-8"))?;
        Ok(stdout
            .replace("\r\n", "\n")
            .trim_end_matches('\n')
            .to_string())
    } else {
        Err(format!(
            "{tool} --version failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
