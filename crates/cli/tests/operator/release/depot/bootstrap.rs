use std::process::Command;

#[test]
fn explicit() {
    let root = tempfile::tempdir().unwrap();
    let generation = "a".repeat(64);
    for (arguments, expected) in [
        (
            vec!["guard", "--generation", &generation],
            "--configuration-marker",
        ),
        (
            vec!["guard", "--configuration-marker", "v1.2.3"],
            "--generation",
        ),
        (
            vec![
                "guard",
                "--configuration-marker",
                "v1.2.3",
                "--generation",
                &generation,
            ],
            "explicit isolated PLUMB_HOME",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .current_dir(root.path())
            .env_remove("PLUMB_HOME")
            .args(arguments)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn selection() {
    let root = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let version = plumb::version!("PLUMB").to_string();
    let binding = crate::marker::prepare(
        root.path(),
        home.path(),
        "[release]\nproduct='probe'\nauthority='https://release.test'\n",
        &version,
    );
    for (marker, generation) in [
        ("v99.0.0", binding["configuration"].as_str().unwrap()),
        (
            version.as_str(),
            "0000000000000000000000000000000000000000000000000000000000000000",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
            .current_dir(root.path())
            .env("PLUMB_HOME", home.path())
            .args([
                "guard",
                "--configuration-marker",
                marker,
                "--generation",
                generation,
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("does not match explicit marker and generation")
        );
    }
}
