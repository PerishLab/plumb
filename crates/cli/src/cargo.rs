use std::process::Command;

pub fn command() -> Command {
    let cargo = &crate::catalog::set::current().stable.cargo;
    configured(&cargo.registry, &cargo.index)
}

fn configured(registry: &str, index: &str) -> Command {
    let mut command = Command::new("cargo");
    command.env(
        format!(
            "CARGO_REGISTRIES_{}_INDEX",
            registry.to_ascii_uppercase().replace('-', "_")
        ),
        index,
    );
    command
}
