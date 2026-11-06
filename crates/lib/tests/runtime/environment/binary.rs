use plumb::config::Contract;

fn contract(key: &str) -> Contract {
    Contract {
        inherit: vec![key.into()],
        managed: Vec::new(),
        reject: Vec::new(),
        bind: Default::default(),
    }
}

#[test]
fn names() {
    for key in ["CARGO_BIN_EXE_helper_alpha", "CARGO_BIN_EXE_helper-alpha"] {
        let held = contract(key)
            .capture([(key.into(), "binary".into())])
            .unwrap();
        assert_eq!(held.get(key), Some("binary"));
        let held = plumb::config::contract("cargo")
            .unwrap()
            .capture([(key.into(), "ambient".into())])
            .unwrap();
        assert_eq!(held.get(key), None);
    }
    let key = "cargo_bin_exe_helper-alpha";
    assert_eq!(contract(key).capture([]).is_ok(), cfg!(windows));
}

#[test]
fn invalid() {
    for key in [
        "BAD-NAME",
        "CARGO-BIN-EXE_helper",
        "CARGO_BIN_EXE_bad=name",
        "CARGO_BIN_EXE_bad\0name",
        "CARGO_BIN_EXE_bad name",
        "CARGO_BIN_EXE_bad*name",
        "CARGO_BIN_EXE_bad/name",
    ] {
        assert!(contract(key).capture([]).is_err(), "{key:?}");
    }
}
