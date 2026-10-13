use super::retired;
use crate::catalog::set;
use crate::shape::Dependencies;
use plumb_cli::Ecosystem;
use std::collections::BTreeMap;
use std::process::Command;

pub fn read(dependencies: &mut Dependencies, root: &std::path::Path) {
    let observed = std::env::var_os("PLUMB_PACKAGES_OBSERVATION").map(|text| {
        text.into_string()
            .map_err(|_| "package observation is not text".to_string())
            .and_then(|text| plumb::packages::Observation::read(root, &text))
    });
    let mut cache = BTreeMap::new();
    let mut blind = Vec::new();
    for dependency in &mut dependencies.held {
        if retired(&dependency.name) {
            continue;
        }
        let key = (dependency.ecosystem, dependency.name.clone());
        let latest = cache
            .entry(key)
            .or_insert_with(|| match dependency.ecosystem {
                Ecosystem::Cargo => fetch(dependency.ecosystem, &dependency.name),
                Ecosystem::Npm => npm(root, &dependency.name, &observed),
            });
        match latest {
            Ok(latest) => dependency.latest = Some(latest.clone()),
            Err(error) => blind.push(error.clone()),
        }
    }
    dependencies.blind.extend(blind);
}

fn npm(
    root: &std::path::Path,
    name: &str,
    observed: &Option<Result<plumb::packages::Observation, String>>,
) -> Result<String, String> {
    match observed {
        Some(Ok(held)) => held
            .latest(name)
            .map(str::to_string)
            .ok_or_else(|| format!("npm observation has no answer for {name}")),
        Some(Err(error)) => Err(error.clone()),
        None => plumb::packages::latest(root, name),
    }
}

fn fetch(ecosystem: Ecosystem, name: &str) -> Result<String, String> {
    let url = match ecosystem {
        Ecosystem::Npm => return Err("npm observation must use packages".into()),
        Ecosystem::Cargo => format!(
            "{}/{}",
            set::CARGO
                .index
                .trim_start_matches("sparse+")
                .trim_end_matches('/'),
            plumb_cli::route(name)
        ),
    };
    let output = probe(&url)
        .output()
        .map_err(|error| format!("cannot read {} stable {name}: {error}", ecosystem.name()))?;
    if !output.status.success() {
        return Err(format!(
            "cannot read {} stable {name}: {}",
            ecosystem.name(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    plumb_cli::cargo(&output.stdout)
        .map_err(|error| format!("cannot read {} stable {name}: {error}", ecosystem.name()))
}

fn probe(url: &str) -> Command {
    let mut command = Command::new("curl");
    command
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--compressed",
            "--retry",
            "2",
            "--connect-timeout",
            "5",
            "--max-time",
            "15",
        ])
        .arg(url);
    command
}

#[cfg(test)]
mod tests {
    #[test]
    fn compressed() {
        let command = super::probe("https://cargo.perish.uk/pl/um/plumb");
        assert!(command.get_args().any(|arg| arg == "--compressed"));
    }
}
