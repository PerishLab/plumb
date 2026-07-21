use std::process::Command;

fn seat() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crate should sit two below the repo")
        .to_path_buf()
}

fn run(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(args)
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn version() {
    assert!(run(&["--version"]).contains("plumb v"));
}

#[test]
fn itself() {
    let out = run(&["doctor", seat().to_str().expect("path should be utf8")]);
    assert!(out.contains("true to the skeleton"), "{out}");
}

#[test]
fn blind() {
    let dir = std::env::temp_dir().join("plumb-blind");
    std::fs::create_dir_all(dir.join(".runseal/wrappers")).expect("fixture should be made");
    std::fs::write(dir.join("negentropy.toml"), "[limit\n").expect("laws should be written");
    let out = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(out.contains("blind:"), "{out}");
    assert!(out.contains("cannot read negentropy.toml"), "{out}");
}
