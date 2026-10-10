use plumb::config::{Binding, Contract, Environment};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub(super) fn capture(
    root: &Path,
    programs: &[String],
    inputs: &[(OsString, OsString)],
) -> Result<Environment, String> {
    let families = programs
        .iter()
        .map(|program| crate::execution::family(program))
        .chain(["probe"])
        .collect::<std::collections::BTreeSet<_>>();
    let mut environment = crate::execution::contract("probe")?.capture([])?;
    for family in &families {
        environment = environment
            .combine(crate::execution::contract(family)?.capture(inputs.iter().cloned())?)?;
    }
    if families.contains("cargo") {
        inspect(root, &environment)?;
    }
    Ok(environment)
}

fn inspect(root: &Path, environment: &Environment) -> Result<(), String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve Cargo root: {error}"))?;
    crate::execution::inspect("cargo", &root, environment)?;
    let home = plumb::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))
        .ok_or_else(|| "Cargo execution has no managed home".to_string())?;
    crate::execution::inspect("cargo", &home.join("tmp/guard"), environment)
}

pub(super) fn managed(
    environment: Environment,
    lease: &super::cargo::Lease,
) -> Result<Environment, String> {
    let mut command = crate::cargo::command();
    command.env("CARGO_TARGET_DIR", &lease.root);
    let bind = command
        .get_envs()
        .filter_map(|(key, value)| value.map(|value| (key, value)))
        .map(|(key, value)| {
            Ok((
                key.to_str().ok_or("managed Cargo key is not text")?.into(),
                Binding::Value(
                    value
                        .to_str()
                        .ok_or("managed Cargo value is not text")?
                        .into(),
                ),
            ))
        })
        .collect::<Result<_, String>>()?;
    environment.combine(
        Contract {
            inherit: Vec::new(),
            managed: Vec::new(),
            reject: Vec::new(),
            bind,
        }
        .capture([])?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed() {
        if std::env::var_os("PLUMB_MIXED_TEST").is_none() {
            let home = tempfile::tempdir().unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "command::guard::precommit::environment::tests::mixed",
                ])
                .env("PLUMB_MIXED_TEST", "child")
                .env("PLUMB_HOME", home.path())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        assert!(
            plumb::config::detached("git")
                .args(["init", "-q"])
                .arg(root.path())
                .status()
                .unwrap()
                .success()
        );
        let mut inputs = std::env::vars_os()
            .filter(|(key, _)| {
                let key = key.to_string_lossy().to_ascii_uppercase();
                !(key.starts_with("CARGO_") || key.starts_with("RUST"))
                    || matches!(key.as_str(), "RUSTUP_HOME" | "RUSTUP_TOOLCHAIN")
            })
            .collect::<Vec<_>>();
        inputs.push(("CARGO_HOME".into(), home.path().as_os_str().to_owned()));
        let programs = ["cargo", "rustc", "node", "pnpm"].map(str::to_string);
        let environment = capture(root.path(), &programs, &inputs).unwrap();
        let lease = super::super::cargo::Lease::new(root.path()).unwrap();
        let environment = managed(environment, &lease).unwrap();
        assert_eq!(environment.get("CARGO_TARGET_DIR"), lease.root.to_str());
        let execution =
            super::super::world::execution("guard/web", environment, root.path(), &programs)
                .unwrap();
        std::fs::write(
            root.path().join("package.json"),
            r#"{"private":true,"scripts":{"probe":"node probe.cjs"}}"#,
        )
        .unwrap();
        std::fs::write(root.path().join("probe.cjs"),
            r#"const {spawnSync}=require('node:child_process'); if(!process.env.CARGO_TARGET_DIR)process.exit(9); const child=spawnSync('cargo',['--version'],{env:{...process.env,npm_config_node_gyp:'parent-generated'}}); process.stdout.write(child.stdout); process.stderr.write(child.stderr); process.exit(child.status ?? 8);"#).unwrap();
        let output = execution
            .output(&["pnpm".into(), "run".into(), "probe".into()])
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("cargo "));
        std::fs::create_dir(root.path().join("src")).unwrap();
        std::fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='mixed'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        std::fs::write(root.path().join("build.rs"),
            r#"fn main(){assert!(std::process::Command::new("node").arg("--version").status().unwrap().success());}"#).unwrap();
        std::fs::write(root.path().join("src/main.rs"),
            r#"fn main(){assert!(std::env::var_os("CARGO_TARGET_DIR").is_some());println!("mixed-world");}"#).unwrap();
        let output = execution
            .output(&["cargo".into(), "run".into(), "--offline".into()])
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("mixed-world"));
        assert!(lease.root.join("debug").is_dir());
        assert!(!root.path().join("target").exists());
        std::fs::create_dir(home.path().join(".cargo")).unwrap();
        std::fs::write(home.path().join("config.toml"), "[build]\njobs=1\n").unwrap();
        assert!(capture(root.path(), &programs, &inputs).is_err());
    }
}
