pub(super) struct Binding<'a> {
    pub execution: Option<&'a plumb::config::Execution>,
    pub probes: &'a std::collections::BTreeMap<String, Vec<plumb::rule::Probe>>,
}

use sha2::{Digest as _, Sha256};

pub(super) struct Fingerprint {
    pub material: String,
    pub proof: String,
}

pub(super) fn digest(
    name: &str,
    input: &str,
    commands: &[Vec<String>],
    binding: &Binding<'_>,
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

pub(super) fn material(
    worlds: &[String],
    packages: &[plumb::packages::Package],
) -> Result<String, String> {
    let bytes = serde_json::to_vec(&("plumb.compilation-world/v2", worlds, packages))
        .map_err(|error| error.to_string())?;
    Ok(plumb::depot::sha(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::cache::transport::{Context, Session};
    use crate::command::guard::precommit::cargo::Lease;
    use std::path::Path;
    use std::process::{Command, Output};

    fn package(version: &str) -> plumb::packages::Package {
        plumb::packages::Package {
            ecosystem: "cargo".into(),
            name: "locus".into(),
            version: version.into(),
        }
    }

    #[test]
    fn selections() {
        let first = material(&["same-execution".into()], &[package("0.1.0")]).unwrap();
        assert_ne!(
            first,
            material(&["same-execution".into()], &[package("0.2.0")]).unwrap()
        );
        assert_ne!(
            first,
            material(&["changed-execution".into()], &[package("0.1.0")]).unwrap()
        );
        assert_eq!(
            first,
            material(&["same-execution".into()], &[package("0.1.0")]).unwrap()
        );
    }
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

    fn cargo(root: &Path, lease: &Lease) -> Output {
        Command::new("cargo")
            .args(["check", "--locked", "--offline"])
            .current_dir(root)
            .env("CARGO_TARGET_DIR", &lease.root)
            .env("CARGO_INCREMENTAL", "0")
            .env_remove("RUSTC_WRAPPER")
            .env_remove("RUSTC_WORKSPACE_WRAPPER")
            .output()
            .unwrap()
    }

    #[test]
    fn retained() {
        if std::env::var_os("PLUMB_NATIVE_TEST").is_none() {
            let home = tempfile::tempdir().unwrap();
            let output = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "command::guard::world::tests::retained",
                    "--nocapture",
                ])
                .env("PLUMB_NATIVE_TEST", "child")
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
        std::fs::create_dir_all(root.path().join("support/src")).unwrap();
        std::fs::write(root.path().join("Cargo.toml"), "[package]\nname='probe'\nversion='0.1.0'\nedition='2024'\n[dependencies]\nsupport={path='support'}\n").unwrap();
        std::fs::write(
            root.path().join("support/Cargo.toml"),
            "[package]\nname='support'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join("support/src/lib.rs"),
            "pub fn value() -> u8 { 1 }\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join("src/lib.rs"),
            "pub fn value() -> u8 { support::value() }\n",
        )
        .unwrap();
        let output = Command::new("cargo")
            .args(["generate-lockfile", "--offline"])
            .current_dir(root.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        git(root.path(), &["add", "."]);
        let lease = Lease::new(root.path()).unwrap();
        let context = Context {
            image: format!("sha256:{}", "e".repeat(64)),
        };
        let first = Session::begin(root.path(), &lease, "same-compile-conditions", &context)
            .unwrap()
            .unwrap();
        let output = cargo(root.path(), &lease);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let sentinel = lease.root.join("debug/retained");
        std::fs::write(&sentinel, "owned compilation material").unwrap();
        first.seal().unwrap();
        std::fs::write(
            root.path().join("src/lib.rs"),
            "pub fn value() -> u8 { support::value() + 1 }\n",
        )
        .unwrap();
        git(root.path(), &["add", "src/lib.rs"]);
        let next = Session::begin(root.path(), &lease, "same-compile-conditions", &context)
            .unwrap()
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&sentinel).unwrap(),
            "owned compilation material"
        );
        let output = cargo(root.path(), &lease);
        let text = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{text}");
        assert!(
            text.contains("Checking probe"),
            "changed root must recheck: {text}"
        );
        assert!(
            !text.contains("Checking support"),
            "unchanged dependency must remain fresh: {text}"
        );
        next.seal().unwrap();
        std::fs::write(
            root.path().join("support/src/lib.rs"),
            "pub fn renamed() -> u8 { 1 }\n",
        )
        .unwrap();
        git(root.path(), &["add", "support/src/lib.rs"]);
        let _invalid = Session::begin(root.path(), &lease, "same-compile-conditions", &context)
            .unwrap()
            .unwrap();
        let output = cargo(root.path(), &lease);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("E0425"));
    }
}
