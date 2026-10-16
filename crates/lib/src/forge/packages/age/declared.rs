use super::super::{fetch, process};
use super::Policy;
use crate::config::{Binding, Contract, Execution};
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[test]
fn declared() {
    if let Some(root) = std::env::var_os("PLUMB_AGE_DECLARED") {
        exercise(Path::new(&root));
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let date = ["node", "-p", "new Date().toISOString()"].map(str::to_string);
    let fresh = String::from_utf8(process::run(root.path(), "pnpm", &date).unwrap()).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let server = std::thread::spawn(move || super::tests::serve(listener, stopped, fresh, true));
    std::fs::write(
        root.path().join(".npmrc"),
        format!("registry={endpoint}/\n"),
    )
    .unwrap();
    std::fs::write(
        root.path().join("package.json"),
        r#"{"name":"declared-fixture","private":true,"dependencies":{"aged-third":"0.1.0"}}"#,
    )
    .unwrap();
    std::fs::write(
        root.path().join("pnpm-workspace.yaml"),
        "packages: []\nminimumReleaseAge: 15\n",
    )
    .unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "packages::age::declared::declared",
            "--nocapture",
        ])
        .env("PLUMB_AGE_DECLARED", root.path())
        .env(
            "NPM_CONFIG_USERCONFIG",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env_remove("WHARF_PACKAGES_TOKEN");
    let result = process::capture(command, "declared fixture");
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
    assert!(result.is_ok(), "{result:?}");
    println!("{}", String::from_utf8(result.unwrap()).unwrap());
}

fn exercise(root: &Path) {
    let lock = [
        "pnpm",
        "install",
        "--lockfile-only",
        "--ignore-scripts",
        "--ignore-pnpmfile",
    ]
    .map(str::to_string);
    process::run(root, "packages", &lock).unwrap();
    let home = tempfile::tempdir().unwrap();
    let policy = Policy::read(root).unwrap();
    assert_eq!(policy.minimum, Some(15));
    let mut flags = policy.flags.clone();
    flags.push(super::cache(home.path()).unwrap());
    let guard = web(root, &policy);
    let typed = pnpm(
        &guard,
        root,
        &flags,
        &["config", "get", "minimumReleaseAge", "--json"],
    );
    assert_eq!(typed.unwrap().trim(), "15");
    fetch::run(root, &flags).unwrap();
    let reused = install(&guard, root, &flags).unwrap();
    assert!(
        reused.contains("supply-chain policies (verified"),
        "guard/web must reuse its preparation's verification under a declared minimum: {reused}"
    );
    std::fs::remove_dir_all(root.join("node_modules")).unwrap();
    std::fs::write(
        root.join("pnpm-workspace.yaml"),
        "packages: []\nminimumReleaseAge: 5256000\n",
    )
    .unwrap();
    let policy = Policy::read(root).unwrap();
    assert_eq!(policy.minimum, Some(5256000));
    let guard = web(root, &policy);
    let denied = fetch::run(root, &flags).unwrap_err();
    assert!(
        denied.contains("MINIMUM_RELEASE_AGE_VIOLATION") && denied.contains("aged-third@0.1.0"),
        "{denied}"
    );
    let denied = install(&guard, root, &flags).unwrap_err();
    assert!(
        denied.contains("MINIMUM_RELEASE_AGE_VIOLATION") && denied.contains("aged-third@0.1.0"),
        "{denied}"
    );
}

fn web(root: &Path, policy: &Policy) -> Execution {
    let inputs = std::env::vars_os().filter(|(key, _)| {
        !key.to_string_lossy()
            .eq_ignore_ascii_case("NPM_CONFIG_USERCONFIG")
    });
    let environment = crate::config::contract("pnpm")
        .unwrap()
        .capture(inputs)
        .unwrap()
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
        .unwrap()
        .combine(policy.environment().unwrap())
        .unwrap();
    Execution::new(environment, &["pnpm".into()], root).unwrap()
}

fn install(guard: &Execution, root: &Path, flags: &[String]) -> Result<String, String> {
    let args = [
        "install",
        "--offline",
        "--frozen-lockfile",
        "--ignore-scripts",
    ];
    pnpm(guard, root, flags, &args)
}

fn pnpm(guard: &Execution, root: &Path, flags: &[String], args: &[&str]) -> Result<String, String> {
    let mut command = guard.command("pnpm").unwrap();
    command.args(flags).args(args).current_dir(root);
    let output = process::capture(command, "guard pnpm")?;
    Ok(String::from_utf8(output).unwrap())
}
