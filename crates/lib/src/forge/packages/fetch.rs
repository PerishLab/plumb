use super::process;
use std::path::Path;

pub fn run(root: &Path, exclusions: &[String]) -> Result<(), String> {
    let staging = tempfile::tempdir().map_err(|error| error.to_string())?;
    let modules = staging.path().join("modules");
    let virtuals = staging.path().join("virtual");
    let mut argv = vec![
        "pnpm".into(),
        "fetch".into(),
        "--frozen-lockfile".into(),
        "--ignore-scripts".into(),
        "--ignore-pnpmfile".into(),
        "--modules-dir".into(),
        modules
            .to_str()
            .ok_or("npm preparation path is not text")?
            .into(),
        "--virtual-store-dir".into(),
        virtuals
            .to_str()
            .ok_or("npm preparation path is not text")?
            .into(),
    ];
    argv.extend_from_slice(exclusions);
    process::run(root, "packages", &argv)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Binding, Contract, Execution};

    fn product(root: &Path) -> Execution {
        let inputs = std::env::vars_os().filter(|(key, _)| {
            !key.to_string_lossy()
                .eq_ignore_ascii_case("NPM_CONFIG_USERCONFIG")
        });
        let environment = crate::config::contract("pnpm")
            .unwrap()
            .capture(inputs)
            .unwrap();
        let environment = environment
            .combine(
                Contract {
                    inherit: Vec::new(),
                    managed: Vec::new(),
                    reject: Vec::new(),
                    bind: [(
                        "NPM_CONFIG_USERCONFIG".into(),
                        Binding::Value(if cfg!(windows) {
                            "NUL".into()
                        } else {
                            "/dev/null".into()
                        }),
                    )]
                    .into(),
                }
                .capture([])
                .unwrap(),
            )
            .unwrap();
        Execution::new(environment, &["pnpm".into()], root).unwrap()
    }

    fn execute(execution: &Execution, root: &Path, args: &[&str]) {
        let mut command = execution.command("pnpm").unwrap();
        command.args(args).current_dir(root);
        let stage = format!("pnpm {}", args.join(" "));
        process::capture(command, &stage).unwrap_or_else(|error| panic!("{stage}: {error}"));
    }

    fn archive(root: &Path, script: &str) {
        let file = std::fs::File::create(root.join("dependency.tgz")).unwrap();
        let writer = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(writer);
        let manifest = serde_json::json!({"name":"fixture-dependency","version":"0.1.0","scripts":{"postinstall":"node probe.cjs dependency"}}).to_string();
        for (name, value) in [
            ("package/package.json", manifest.as_str()),
            ("package/probe.cjs", script),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(value.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            archive
                .append_data(&mut header, name, value.as_bytes())
                .unwrap();
        }
        archive.into_inner().unwrap().finish().unwrap();
    }

    #[test]
    fn lifecycle() {
        if std::env::var_os("PLUMB_FETCH_FIXTURE").is_none() {
            let home = tempfile::tempdir().unwrap();
            let credential = home.path().join("synthetic.npmrc");
            std::fs::write(
                &credential,
                "//npm.pkg.github.com/:_authToken=SYNTHETIC_ONLY_SAFE\n",
            )
            .unwrap();
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "packages::fetch::tests::lifecycle",
                    "--nocapture",
                ])
                .env("PLUMB_FETCH_FIXTURE", "child")
                .env("NPM_CONFIG_USERCONFIG", credential)
                .env("WHARF_PACKAGES_TOKEN", "SYNTHETIC_ONLY_ENV");
            process::capture(command, "lifecycle fixture").unwrap();
            return;
        }
        let root = tempfile::tempdir().unwrap();
        let events = root.path().join("events.jsonl");
        let script = format!(
            "const fs=require('node:fs');const p=process.env.NPM_CONFIG_USERCONFIG||process.env.npm_config_userconfig;fs.appendFileSync({},JSON.stringify({{phase:process.argv[2],token:!!process.env.WHARF_PACKAGES_TOKEN,canary:!!p&&fs.readFileSync(p,'utf8').includes('SYNTHETIC_ONLY_SAFE')}})+'\\n');",
            serde_json::to_string(&events).unwrap()
        );
        archive(root.path(), &script);
        std::fs::write(root.path().join("probe.cjs"), &script).unwrap();
        let scripts = ["preinstall", "install", "postinstall", "prepare"]
            .into_iter()
            .map(|phase| (phase, format!("node probe.cjs {phase}")))
            .collect::<std::collections::BTreeMap<_, _>>();
        std::fs::write(root.path().join("package.json"), serde_json::json!({"name":"fixture","private":true,"dependencies":{"fixture-dependency":"file:dependency.tgz"},"scripts":scripts}).to_string()).unwrap();
        std::fs::write(
            root.path().join("pnpm-workspace.yaml"),
            "packages: []\nsideEffectsCache: false\nallowBuilds:\n  'fixture-dependency@file:dependency.tgz': true\n",
        )
        .unwrap();
        std::fs::write(root.path().join(".pnpmfile.cjs"), "require('node:child_process').execFileSync('node',['probe.cjs','pnpmfile']);module.exports={hooks:{}};").unwrap();
        let product = product(root.path());
        execute(
            &product,
            root.path(),
            &["install", "--lockfile-only", "--ignore-scripts"],
        );
        std::fs::remove_file(&events).unwrap();
        execute(
            &product,
            root.path(),
            &["install", "--offline", "--frozen-lockfile"],
        );
        let baseline = std::fs::read_to_string(&events).unwrap();
        assert!(
            baseline.contains("dependency"),
            "baseline install must execute the approved dependency: {baseline}"
        );
        std::fs::remove_file(&events).unwrap();
        std::fs::remove_dir_all(root.path().join("node_modules")).unwrap();
        run(root.path(), &super::super::exclusions(root.path()).unwrap()).unwrap();
        assert!(
            !events.exists(),
            "authenticated fetch must not execute repository code"
        );
        execute(
            &product,
            root.path(),
            &["install", "--offline", "--frozen-lockfile"],
        );
        let values = std::fs::read_to_string(events)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert!(
            values
                .iter()
                .all(|value| value["token"] == false && value["canary"] == false)
        );
        assert_eq!(
            values
                .iter()
                .filter(|value| value["phase"] == "dependency")
                .count(),
            1,
            "events={values:?} lock={}",
            std::fs::read_to_string(root.path().join("pnpm-lock.yaml")).unwrap()
        );
        let phases = values
            .iter()
            .filter_map(|value| value["phase"].as_str())
            .filter(|phase| ["preinstall", "install", "postinstall", "prepare"].contains(phase))
            .collect::<Vec<_>>();
        assert_eq!(phases, ["preinstall", "install", "postinstall", "prepare"]);
    }

    #[test]
    fn shadowed() {
        let root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let default = tempfile::tempdir().unwrap();
        archive(root.path(), "");
        std::fs::write(root.path().join("package.json"), serde_json::json!({"name":"fixture","private":true,"dependencies":{"fixture-dependency":"file:dependency.tgz"}}).to_string()).unwrap();
        std::fs::write(root.path().join("pnpm-workspace.yaml"), "packages: []\n").unwrap();
        let plain = |args: &[&str]| {
            let mut command = std::process::Command::new("pnpm");
            command
                .args(args)
                .current_dir(root.path())
                .env("NPM_CONFIG_USERCONFIG", "/dev/null")
                .env("XDG_CACHE_HOME", default.path());
            process::capture(command, "plain pnpm").unwrap()
        };
        plain(&["install", "--lockfile-only", "--ignore-scripts"]);
        let mut flags = super::super::exclusions(root.path()).unwrap();
        flags.push(super::super::cache(home.path()).unwrap());
        run(root.path(), &flags).unwrap();
        plain(&["install", "--frozen-lockfile", "--ignore-scripts"]);
        std::fs::remove_dir_all(root.path().join("node_modules")).unwrap();
        let product = product(root.path());
        let mut command = product.command("pnpm").unwrap();
        command
            .args(&flags)
            .args([
                "install",
                "--offline",
                "--frozen-lockfile",
                "--ignore-scripts",
            ])
            .current_dir(root.path());
        let output =
            String::from_utf8(process::capture(command, "guard install").unwrap()).unwrap();
        assert!(
            output.contains("supply-chain policies (verified"),
            "Guard's install must reuse its preparation's verification: {output}"
        );
    }
}
