#[test]
fn untouched() {
    if std::env::var_os("PLUMB_TEST_AMBIENT").is_some() {
        return;
    }
    let home = tempfile::tempdir().unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .arg("command::packages::follow::tests::")
        .env("PLUMB_TEST_AMBIENT", "1")
        .env("PLUMB_HOME", home.path())
        .env_remove("PLUMB_TEST_HOME")
        .env_remove("PLUMB_TEST_RECOVERY")
        .env_remove("CI");
    for (key, _) in std::env::vars() {
        let ambient = key.starts_with("CARGO_") || key.starts_with("RUST");
        if ambient && !["CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN"].contains(&key.as_str()) {
            command.env_remove(key);
        }
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let left = std::fs::read_dir(home.path().join("tmp"))
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    assert!(
        left.is_empty(),
        "packages tests staged into the ambient PLUMB_HOME: {left:?}"
    );
}
