use plumb::config::{Binding, Cache, Contract, Execution};

#[test]
fn absent() {
    let root = tempfile::tempdir().unwrap();
    let contract: Contract = toml::from_str("inherit=[]\nmanaged=[]\nreject=[]").unwrap();
    let execution = Execution::new(contract.capture([]).unwrap(), &[], root.path()).unwrap();
    let directory = root.path().join("cache");
    assert!(Cache::start(&execution, &directory).unwrap().is_none());
    assert!(!directory.exists());
}

#[test]
#[ignore = "explicit integration with the canonical image's exact sccache binary"]
fn actual() {
    let root = tempfile::tempdir().unwrap();
    let contract: Contract = toml::from_str("inherit=['PATH','HOME','USERPROFILE','CARGO_HOME','RUSTUP_HOME','RUSTUP_TOOLCHAIN','SystemRoot','SYSTEMROOT','WINDIR','PATHEXT','TEMP','TMP']\nmanaged=[]\nreject=[]\n[bind]\nRUSTC_WRAPPER={tool='sccache'}\nCARGO_INCREMENTAL='0'").unwrap();
    let execution = Execution::new(
        plumb::config::environment(&contract).unwrap(),
        &["rustc".into()],
        root.path(),
    )
    .unwrap();
    let directory = root.path().join("cache");
    let first = Cache::start(&execution, &directory).unwrap().unwrap();
    let second = Cache::start(&execution, &directory).unwrap().unwrap();
    let mut one = execution.command("sccache").unwrap();
    let mut two = execution.command("sccache").unwrap();
    first.apply(&mut one);
    second.apply(&mut two);
    let port = |command: &std::process::Command| {
        command
            .get_envs()
            .find(|(key, _)| *key == "SCCACHE_SERVER_PORT")
            .unwrap()
            .1
            .unwrap()
            .to_owned()
    };
    assert_ne!(port(&one), port(&two));
    std::fs::write(
        root.path().join("lib.rs"),
        "pub fn answer() -> u32 { 42 }\n",
    )
    .unwrap();
    let compiler = execution.tools().unwrap()["rustc"].path.clone();
    one.arg(&compiler).args([
        "--crate-name",
        "fixture",
        "--crate-type",
        "lib",
        "--emit=dep-info,link",
        "lib.rs",
        "--out-dir",
        ".",
    ]);
    two.arg(&compiler).args([
        "--crate-name",
        "fixture",
        "--crate-type",
        "lib",
        "--emit=dep-info,link",
        "lib.rs",
        "--out-dir",
        ".",
    ]);
    assert!(one.status().unwrap().success());
    let original = std::fs::read(root.path().join("libfixture.rlib")).unwrap();
    std::fs::remove_file(root.path().join("libfixture.rlib")).unwrap();
    drop(first);
    assert!(two.status().unwrap().success());
    assert!(
        original == std::fs::read(root.path().join("libfixture.rlib")).unwrap(),
        "same compilation inputs produce identical bytes across owned services"
    );
    let mut stats = execution.command("sccache").unwrap();
    second.apply(&mut stats);
    let output = stats
        .args(["--show-stats", "--stats-format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stats: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        stats["stats"]["cache_hits"]["counts"]["Rust"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "{stats}"
    );
    drop(second);
    let mut stopped = execution.command("sccache").unwrap();
    stopped
        .env("SCCACHE_SERVER_PORT", port(&one))
        .arg("--stop-server");
    assert!(!stopped.output().unwrap().status.success());
}

fn invoke(execution: &Execution, cache: &Cache<'_>, argv: &[&str]) -> std::process::Output {
    let mut command = execution.command(argv[0]).unwrap();
    cache.apply(&mut command);
    command.args(&argv[1..]).output().unwrap()
}

struct Case {
    name: &'static str,
    lint: &'static str,
    changed: bool,
    expected: i32,
}

#[test]
#[ignore = "explicit integration with the canonical image's exact Clippy and sccache binaries"]
fn clippy() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("src")).unwrap();
    std::fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname='probe'\nversion='0.0.0'\nedition='2024'\n",
    )
    .unwrap();
    let source = "pub fn choose(flag: bool) -> bool { if flag { true } else { false } }\npub fn value() -> u8 { 1 }\n";
    std::fs::write(root.path().join("src/lib.rs"), source).unwrap();
    let contract = plumb::config::contract("cargo").unwrap();
    let inherited = std::env::vars_os()
        .filter(|(key, _)| contract.inherit.iter().any(|name| key == name.as_str()));
    let environment = contract
        .capture(inherited)
        .unwrap()
        .combine(
            Contract {
                inherit: vec![],
                managed: vec![],
                reject: vec![],
                bind: [
                    (
                        "RUSTC".into(),
                        Binding::Tool {
                            tool: "rustc".into(),
                        },
                    ),
                    (
                        "RUSTC_WRAPPER".into(),
                        Binding::Tool {
                            tool: "sccache".into(),
                        },
                    ),
                    ("CARGO_INCREMENTAL".into(), Binding::Value("0".into())),
                    (
                        "CARGO_TARGET_DIR".into(),
                        Binding::Value(root.path().join("target").display().to_string()),
                    ),
                ]
                .into(),
            }
            .capture([])
            .unwrap(),
        )
        .unwrap();
    let execution =
        Execution::new(environment, &["cargo".into(), "rustc".into()], root.path()).unwrap();
    assert!(execution.tools().unwrap()["rustc"].path.is_absolute());
    let directory = root.path().join("compiler");
    let cache = Cache::start(&execution, &directory).unwrap().unwrap();
    for Case {
        name,
        lint,
        changed,
        expected,
    } in [
        Case {
            name: "cold",
            lint: "-W",
            changed: false,
            expected: 0,
        },
        Case {
            name: "repeat",
            lint: "-W",
            changed: false,
            expected: 0,
        },
        Case {
            name: "changed",
            lint: "-W",
            changed: true,
            expected: 0,
        },
        Case {
            name: "changed-repeat",
            lint: "-W",
            changed: true,
            expected: 0,
        },
        Case {
            name: "deny",
            lint: "-D",
            changed: true,
            expected: 101,
        },
    ] {
        if root.path().join("target").exists() {
            std::fs::remove_dir_all(root.path().join("target")).unwrap();
        }
        if changed {
            std::fs::write(
                root.path().join("src/lib.rs"),
                source.replace("{ 1 }", "{ 2 }"),
            )
            .unwrap();
        }
        assert!(
            invoke(&execution, &cache, &["sccache", "--zero-stats"])
                .status
                .success()
        );
        let output = invoke(
            &execution,
            &cache,
            &[
                "cargo",
                "clippy",
                "--offline",
                "--lib",
                "--",
                lint,
                "clippy::needless_bool",
            ],
        );
        assert_eq!(
            output.status.code(),
            Some(expected),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("needless_bool"),
            "{name}: diagnostics must survive cache hits"
        );
        let output = invoke(
            &execution,
            &cache,
            &["sccache", "--show-stats", "--stats-format", "json"],
        );
        assert!(output.status.success());
        let stats: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let hits = stats["stats"]["cache_hits"]["counts"]["Rust"]
            .as_u64()
            .unwrap_or(0);
        let misses = stats["stats"]["cache_misses"]["counts"]["Rust"]
            .as_u64()
            .unwrap_or(0);
        println!("{name}: hits={hits}, misses={misses}, exit={expected}");
        if name == "repeat" || name == "changed-repeat" {
            assert!(hits > 0, "{name}: {stats}");
        } else {
            assert_eq!(hits, 0, "{name}: {stats}");
            if expected == 0 {
                assert!(misses > 0, "{name}: {stats}");
            } else {
                assert!(
                    stats["stats"]["compile_fails"].as_u64().unwrap_or(0) > 0,
                    "{stats}"
                );
            }
        }
    }
    std::fs::remove_dir_all(root.path().join("target")).unwrap();
    let mut plain = execution.command("cargo").unwrap();
    plain.env_remove("RUSTC_WRAPPER").args([
        "clippy",
        "--offline",
        "--lib",
        "--",
        "-D",
        "clippy::needless_bool",
    ]);
    let output = plain.output().unwrap();
    assert_eq!(output.status.code(), Some(101));
    assert!(String::from_utf8_lossy(&output.stderr).contains("needless_bool"));
}
