pub(crate) mod cargo;

use plumb::config::{Contract, Environment, Execution};
use plumb::rule::{Observation, Probe};
use std::path::Path;

pub(crate) fn contract(name: &str) -> Result<Contract, String> {
    plumb::config::contract(name)
}

pub(crate) fn environment(name: &str) -> Result<Environment, String> {
    plumb::config::environment(&contract(name)?)
}

pub(crate) fn family(program: &str) -> &'static str {
    match program {
        "cargo" | "rustc" | "rustup" => "cargo",
        "node" | "pnpm" => "pnpm",
        "docker" => "oci",
        "regctl" => "registry",
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
