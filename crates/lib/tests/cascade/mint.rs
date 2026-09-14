use plumb::{config::Cascade, rig::Mint};

#[test]
fn layered() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("token");
    std::fs::write(&file, " file-value\n").unwrap();
    let lower = toml::from_str::<plumb::rig::MintPartial>(&format!(
        "account='account'\ntoken_file={}\n",
        toml::Value::String(file.to_string_lossy().into_owned())
    ))
    .unwrap();
    let mut held = Mint::default().merge(lower);
    held.load().unwrap();
    assert_eq!(held.token, "file-value");
    let override_ = Mint::lookup("PLUMB_AUTHORITY", &|key| match key {
        "PLUMB_AUTHORITY_TOKEN" => Some("direct-value".into()),
        "PLUMB_AUTHORITY_TOKEN_FILE" => {
            Some(root.path().join("absent").to_string_lossy().into_owned())
        }
        _ => None,
    })
    .unwrap();
    let mut held = held.merge(override_);
    held.load().unwrap();
    assert_eq!(held.token, "direct-value");
    assert_eq!(held.account, "account");
}

#[test]
fn refusal() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("token");
    let mut held = Mint {
        file: file.clone(),
        ..Default::default()
    };
    assert!(held.load().unwrap_err().contains("cannot read"));
    std::fs::write(&file, " \n").unwrap();
    assert!(held.load().unwrap_err().contains("holds no secret"));
    assert!(held.token.is_empty());
}
