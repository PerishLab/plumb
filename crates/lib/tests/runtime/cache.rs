use plumb::config::{Cache, Contract, Execution};

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
    first.finish().unwrap();
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
    second.finish().unwrap();
    let mut stopped = execution.command("sccache").unwrap();
    stopped
        .env("SCCACHE_SERVER_PORT", port(&one))
        .arg("--stop-server");
    assert!(!stopped.output().unwrap().status.success());
}
