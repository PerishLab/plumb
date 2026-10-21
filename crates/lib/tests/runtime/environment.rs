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
fn credential() {
    let inputs = [
        ("NPM_CONFIG_USERCONFIG".into(), "npmrc".into()),
        ("WHARF_PACKAGES_TOKEN".into(), "read".into()),
        ("GH_TOKEN".into(), "write".into()),
    ];
    let packages = plumb::config::contract("packages").unwrap();
    let held = packages.capture(inputs.clone()).unwrap();
    assert_eq!(held.get("NPM_CONFIG_USERCONFIG"), Some("npmrc"));
    assert_eq!(held.get("WHARF_PACKAGES_TOKEN"), Some("read"));
    assert_eq!(held.get("GH_TOKEN"), None);
    let pnpm = plumb::config::contract("pnpm").unwrap();
    assert_eq!(
        pnpm.capture(inputs).unwrap().get("WHARF_PACKAGES_TOKEN"),
        None
    );
    for key in [
        "NPM_CONFIG_REGISTRY",
        "npm_config__authToken",
        "PNPM_CONFIG",
        "NODE_OPTIONS",
    ] {
        assert!(
            packages.capture([(key.into(), "override".into())]).is_err(),
            "{key}"
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

#[test]
fn compiler() {
    let held = plumb::config::contract("cargo")
        .unwrap()
        .capture([
            ("CARGO_INCREMENTAL".into(), "0".into()),
            ("RUSTC_WRAPPER".into(), "/opt/sccache".into()),
        ])
        .unwrap();
    assert!(held.get("CARGO_INCREMENTAL").is_none());
    assert!(held.get("RUSTC_WRAPPER").is_none());
}

#[test]
#[cfg(target_os = "linux")]
fn profile() {
    use object::{Object as _, ObjectSection as _, ObjectSymbol as _};
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("src")).unwrap();
    std::fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname='profile-probe'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::write(root.path().join("src/lib.rs"), "#[unsafe(no_mangle)]\npub extern \"C\" fn profile_symbol() -> u8 { 42 }\n#[test]\nfn assertions() { assert!(cfg!(debug_assertions)); assert_eq!(profile_symbol(), 42); }\n").unwrap();
    let contract = plumb::config::contract("cargo").unwrap();
    for (key, value) in [
        ("CARGO_PROFILE_DEV_DEBUG", "0"),
        ("CARGO_PROFILE_TEST_DEBUG", "0"),
        ("CARGO_PROFILE_DEV_STRIP", "debuginfo"),
        ("CARGO_PROFILE_TEST_STRIP", "debuginfo"),
    ] {
        let held = contract.capture([]).unwrap();
        assert_eq!(held.get(key), Some(value));
        assert!(contract.capture([(key.into(), "override".into())]).is_err());
    }
    let inputs = std::env::vars_os()
        .filter(|(key, _)| contract.inherit.iter().any(|name| key == name.as_str()));
    let execution = plumb::config::Execution::new(
        contract.capture(inputs).unwrap(),
        &["cargo".into()],
        root.path(),
    )
    .unwrap();
    let evidence: serde_json::Value =
        serde_json::from_slice(&execution.evidence().unwrap()).unwrap();
    assert_eq!(evidence[0]["CARGO_PROFILE_DEV_DEBUG"], "0");
    assert_eq!(evidence[0]["CARGO_PROFILE_TEST_STRIP"], "debuginfo");
    let output = execution
        .command("cargo")
        .unwrap()
        .args(["test", "--offline", "--message-format=json"])
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let artifacts = String::from_utf8(output.stdout).unwrap();
    let artifact = artifacts
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|row| row["executable"].is_string())
        .unwrap();
    assert_eq!(artifact["profile"]["debuginfo"], 0);
    assert_eq!(artifact["profile"]["opt_level"], "0");
    assert_eq!(artifact["profile"]["debug_assertions"], true);
    let path = artifact["executable"].as_str().unwrap();
    let bytes = std::fs::read(path).unwrap();
    let object = object::File::parse(bytes.as_slice()).unwrap();
    assert!(!object.sections().any(|section| {
        let name = section.name().unwrap_or_default();
        name.starts_with(".debug_") && name != ".debug_gdb_scripts"
    }));
    assert!(
        object
            .symbols()
            .any(|symbol| symbol.name().unwrap_or_default() == "profile_symbol")
    );
    std::fs::write(
        root.path().join("src/lib.rs"),
        "#[test]\nfn assertions() { debug_assert!(false); }\n",
    )
    .unwrap();
    let failed = execution
        .command("cargo")
        .unwrap()
        .args(["test", "--offline"])
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(
        !failed.status.success(),
        "debug assertion failures remain failures"
    );
}

#[test]
#[cfg(not(target_os = "linux"))]
fn profile() {
    let held = plumb::config::contract("cargo")
        .unwrap()
        .capture([])
        .unwrap();
    assert_eq!(held.get("CARGO_PROFILE_DEV_DEBUG"), None);
    assert_eq!(held.get("CARGO_PROFILE_TEST_STRIP"), None);
}
