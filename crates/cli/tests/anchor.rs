use std::process::Command;

fn run(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(args)
        .output()
        .expect("plumb should run");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn missing() {
    let dir = std::env::temp_dir().join("plumb-anchorless");
    std::fs::create_dir_all(dir.join("crates/other/src")).expect("fixture should be made");
    std::fs::write(dir.join(".gitignore"), "target/\n").expect("ignore should be written");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/other\"]\n",
    )
    .expect("manifest should be written");
    std::fs::write(
        dir.join("crates/other/Cargo.toml"),
        "[package]\nname = \"other\"\n",
    )
    .expect("manifest should be written");
    let bare = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(
        bare.contains("no crate is named plumb-anchorless, the anchor is missing"),
        "{bare}"
    );
}

#[test]
fn seated() {
    let dir = std::env::temp_dir().join("plumb-anchored");
    std::fs::create_dir_all(dir.join("crates/anchor/src")).expect("fixture should be made");
    std::fs::create_dir_all(dir.join("crates/other/src")).expect("fixture should be made");
    std::fs::write(dir.join(".gitignore"), "target/\n").expect("ignore should be written");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/anchor\", \"crates/other\"]\n",
    )
    .expect("manifest should be written");
    std::fs::write(
        dir.join("crates/anchor/Cargo.toml"),
        "[package]\nname = \"plumb-anchored\"\n",
    )
    .expect("manifest should be written");
    std::fs::write(
        dir.join("crates/other/Cargo.toml"),
        "[package]\nname = \"other\"\n",
    )
    .expect("manifest should be written");
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(!held.contains("anchor is missing"), "{held}");

    std::fs::write(
        dir.join("crates/anchor/src/lib.rs"),
        "#[derive(plumb::config::Cascade)]\nstruct Config;\n",
    )
    .expect("source should be written");
    let seated = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!seated.contains("outside the anchor"), "{seated}");
}

#[test]
fn strayed() {
    let dir = std::env::temp_dir().join("plumb-strayed");
    std::fs::create_dir_all(dir.join("crates/anchor/src")).expect("fixture should be made");
    std::fs::create_dir_all(dir.join("crates/other/src")).expect("fixture should be made");
    std::fs::write(dir.join(".gitignore"), "target/\n").expect("ignore should be written");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/anchor\", \"crates/other\"]\n",
    )
    .expect("manifest should be written");
    std::fs::write(
        dir.join("crates/anchor/Cargo.toml"),
        "[package]\nname = \"plumb-strayed\"\n",
    )
    .expect("manifest should be written");
    std::fs::write(
        dir.join("crates/other/Cargo.toml"),
        "[package]\nname = \"other\"\n",
    )
    .expect("manifest should be written");
    std::fs::write(
        dir.join("crates/other/src/lib.rs"),
        "#[derive(plumb::config::Cascade)]\nstruct Config;\n",
    )
    .expect("source should be written");
    let outside = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(
        outside.contains("Cascade derives in crates/other, outside the anchor"),
        "{outside}"
    );

    std::fs::write(
        dir.join("crates/other/Cargo.toml"),
        "[package]\nname = \"other\"\n[lib]\nproc-macro = true\n",
    )
    .expect("manifest should be written");
    let exempt = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!exempt.contains("outside the anchor"), "{exempt}");
}

#[test]
fn homed() {
    let base = std::env::temp_dir().join("plumb-homed-fixture");
    let home = base.join("plumb-homed");
    let seat = base.join("seats/line");
    std::fs::create_dir_all(home.join(".git/worktrees/line")).expect("fixture should be made");
    std::fs::write(home.join(".git/worktrees/line/commondir"), "../..\n")
        .expect("commondir should be written");
    std::fs::create_dir_all(seat.join("crates/anchor/src")).expect("fixture should be made");
    std::fs::create_dir_all(seat.join("crates/other/src")).expect("fixture should be made");
    std::fs::write(
        seat.join(".git"),
        format!(
            "gitdir: {}\n",
            home.join(".git/worktrees/line")
                .to_str()
                .expect("path should be utf8")
        ),
    )
    .expect("git pointer should be written");
    std::fs::write(seat.join(".gitignore"), "target/\n").expect("ignore should be written");
    std::fs::write(
        seat.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/anchor\", \"crates/other\"]\n",
    )
    .expect("manifest should be written");
    std::fs::write(
        seat.join("crates/anchor/Cargo.toml"),
        "[package]\nname = \"plumb-homed\"\n",
    )
    .expect("manifest should be written");
    std::fs::write(
        seat.join("crates/other/Cargo.toml"),
        "[package]\nname = \"other\"\n",
    )
    .expect("manifest should be written");
    let held = run(&["doctor", seat.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&base).expect("fixture should be swept");
    assert!(!held.contains("anchor is missing"), "{held}");
}
