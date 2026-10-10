use plumb::config::Contract;

#[test]
fn executable() {
    let root = tempfile::tempdir().unwrap();
    let file = if cfg!(windows) { "probe.exe" } else { "probe" };
    std::fs::copy(std::env::current_exe().unwrap(), root.path().join(file)).unwrap();
    let contract: Contract =
        toml::from_str("inherit=['PATH','PATHEXT']\nmanaged=[]\nreject=[]\n").unwrap();
    for key in ["PATH", "Path"] {
        let environment = contract
            .capture([
                (key.into(), root.path().as_os_str().to_owned()),
                ("PATHEXT".into(), ".EXE".into()),
            ])
            .unwrap();
        let execution = plumb::config::Execution::new(environment, &["probe".into()], root.path());
        assert_eq!(execution.is_ok(), key == "PATH" || cfg!(windows));
        if let Ok(execution) = execution {
            assert!(execution.command("probe").is_ok());
        }
    }
}

fn contract() -> Contract {
    toml::from_str(
        "inherit=['PATH','SystemRoot']\nmanaged=['CARGO_TARGET_*']\nreject=['CARGO_*','RUST*']\n[bind]\nRUSTC_WRAPPER={tool='sccache'}\nCARGO_INCREMENTAL='0'\n",
    )
    .unwrap()
}

#[test]
fn inherited() {
    let held = contract()
        .capture([
            ("Path".into(), "tools".into()),
            ("SYSTEMROOT".into(), "system".into()),
        ])
        .unwrap();
    assert_eq!(held.get("PATH"), cfg!(windows).then_some("tools"));
    assert_eq!(held.get("SystemRoot"), cfg!(windows).then_some("system"));
    let held = contract()
        .capture([("PATH".into(), "tools".into())])
        .unwrap();
    assert_eq!(held.get("PATH"), Some("tools"));
    assert_eq!(held.get("Path"), cfg!(windows).then_some("tools"));
}

#[test]
fn refused() {
    assert!(
        contract()
            .capture([("CARGO_PROFILE_DEV_DEBUG".into(), "0".into())])
            .is_err()
    );
    let result = contract().capture([("cargo_profile_dev_debug".into(), "0".into())]);
    assert_eq!(result.is_err(), cfg!(windows));
    let result = contract().capture([("cargo_target_dir".into(), "temporary".into())]);
    assert!(result.is_ok());
    assert!(result.unwrap().get("CARGO_TARGET_DIR").is_none());
}

#[test]
fn bound() {
    let result = contract().capture([("cargo_incremental".into(), "1".into())]);
    assert_eq!(result.is_err(), cfg!(windows));
    let held = contract()
        .capture([("rustc_wrapper".into(), "resolved-tool".into())])
        .unwrap();
    assert_eq!(
        held.get("RUSTC_WRAPPER"),
        Some(if cfg!(windows) {
            "resolved-tool"
        } else {
            "sccache"
        })
    );
}

#[test]
fn declarations() {
    let mixed: Contract =
        toml::from_str("inherit=['Path']\nmanaged=[]\nreject=[]\n[bind]\nPATH='fixed'\n").unwrap();
    assert_eq!(mixed.capture([]).is_err(), cfg!(windows));
    let duplicate: Contract =
        toml::from_str("inherit=[]\nmanaged=[]\nreject=[]\n[bind]\nPATH='one'\nPath='two'\n")
            .unwrap();
    assert_eq!(duplicate.capture([]).is_err(), cfg!(windows));
}

#[test]
fn composition() {
    let cargo = plumb::config::contract("cargo").unwrap();
    let pnpm = plumb::config::contract("pnpm").unwrap();
    let inputs = [
        ("PATH".into(), "tools".into()),
        ("RUSTUP_HOME".into(), "rust".into()),
        ("PNPM_HOME".into(), "node".into()),
        ("VCToolsInstallDir".into(), "native".into()),
        ("CARGO_TARGET_DIR".into(), "ambient".into()),
    ];
    let held = cargo
        .capture(inputs.clone())
        .unwrap()
        .combine(pnpm.capture(inputs).unwrap())
        .unwrap();
    assert_eq!(held.get("RUSTUP_HOME"), Some("rust"));
    assert_eq!(held.get("PNPM_HOME"), Some("node"));
    assert_eq!(held.get("VCToolsInstallDir"), Some("native"));
    assert_eq!(held.get("CARGO_TARGET_DIR"), None);
    for key in [
        "CARGO_PROFILE_DEV_DEBUG",
        "NODE_OPTIONS",
        "NPM_CONFIG_NODE_GYP",
    ] {
        let inputs = [(key.into(), "override".into())];
        assert!(
            cargo
                .capture(inputs.clone())
                .and_then(|held| { held.combine(pnpm.capture(inputs)?) })
                .is_err()
        );
    }
}

#[test]
fn conflicts() {
    let capture = |binding: &str| {
        toml::from_str::<Contract>(&format!(
            "inherit=[]\nmanaged=[]\nreject=[]\n[bind]\nMODE={binding}\n"
        ))
        .unwrap()
        .capture([])
        .unwrap()
    };
    assert!(capture("'one'").combine(capture("'two'")).is_err());
    assert!(capture("'tool'").combine(capture("{tool='tool'}")).is_err());
    assert!(
        capture("{tool='one'}")
            .combine(capture("{tool='two'}"))
            .is_err()
    );
    let held = capture("'one'").combine(capture("'one'")).unwrap();
    assert_eq!(held.get("MODE"), Some("one"));
}
