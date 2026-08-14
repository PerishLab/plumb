use super::{Dependencies, Ecosystem};
use crate::rules::RULES;
use std::collections::BTreeMap;
use std::process::Command;

pub fn read(dependencies: &mut Dependencies) {
    let mut cache = BTreeMap::new();
    let mut blind = Vec::new();
    for dependency in &mut dependencies.held {
        if RULES
            .retired
            .iter()
            .any(|(name, _)| name == &dependency.name)
        {
            continue;
        }
        let key = (dependency.ecosystem, dependency.name.clone());
        let latest = cache
            .entry(key)
            .or_insert_with(|| fetch(dependency.ecosystem, &dependency.name));
        match latest {
            Ok(latest) => dependency.latest = Some(latest.clone()),
            Err(error) => blind.push(error.clone()),
        }
    }
    dependencies.blind.extend(blind);
}

fn fetch(ecosystem: Ecosystem, name: &str) -> Result<String, String> {
    let url = match ecosystem {
        Ecosystem::Cargo => format!(
            "{}/{}",
            RULES
                .stable
                .cargo
                .index
                .trim_start_matches("sparse+")
                .trim_end_matches('/'),
            plumb_cli::route(name)
        ),
    };
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--retry",
            "2",
            "--connect-timeout",
            "5",
            "--max-time",
            "15",
        ])
        .arg(&url)
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
