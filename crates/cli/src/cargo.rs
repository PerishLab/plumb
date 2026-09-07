use std::process::Command;

pub fn command() -> Command {
    let cargo = &crate::catalog::set::current().stable.cargo;
    configured(&cargo.registry, &cargo.index)
}

pub fn configure(command: &mut Command) {
    let cargo = &crate::catalog::set::current().stable.cargo;
    for (key, value) in configured(&cargo.registry, &cargo.index).get_envs() {
        if let Some(value) = value {
            command.env(key, value);
        }
    }
}

fn configured(registry: &str, index: &str) -> Command {
    let mut command = plumb::config::detached("cargo");
    command.env(
        format!(
            "CARGO_REGISTRIES_{}_INDEX",
            registry.to_ascii_uppercase().replace('-', "_")
        ),
        index,
    );
    command
}
