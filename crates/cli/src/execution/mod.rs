pub(crate) mod cargo;

use plumb::config::{Contract, Environment, Execution};
use plumb::rule::{Observation, Probe};
use std::path::Path;

pub(crate) fn contract(name: &str) -> Result<Contract, String> {
    crate::catalog::set::read("workflow")?
        .get("execution")
        .and_then(|held| held.get(name))
        .ok_or_else(|| format!("rules/workflow.toml must declare execution.{name}"))?
        .clone()
        .try_into()
        .map_err(|error| format!("invalid {name} execution contract: {error}"))
}

pub(crate) fn environment(name: &str) -> Result<Environment, String> {
    plumb::config::environment(&contract(name)?)
}

pub(crate) fn family(program: &str) -> &'static str {
    match program {
        "cargo" | "rustc" | "rustup" => "cargo",
        "node" | "pnpm" | "corepack" => "pnpm",
        _ => "probe",
    }
}

pub(crate) fn inspect(name: &str, root: &Path, environment: &Environment) -> Result<(), String> {
    if name == "cargo" {
        cargo::inspect(root, environment)?;
    }
    Ok(())
}

pub(crate) fn observe(probe: &Probe, root: &Path) -> Result<Observation, String> {
    probe.validate()?;
    let family = family(&probe.argv[0]);
    let environment = environment(family)?;
    inspect(family, root, &environment)?;
    let execution = Execution::new(environment, &[probe.argv[0].clone()], root)?;
    probe.run(&execution)
}
