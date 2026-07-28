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
fn governs() {
    let bare = std::env::temp_dir().join("plumb-ungoverned");
    std::fs::create_dir_all(&bare).expect("fixture should be made");
    std::fs::write(bare.join("README.md"), "# bare\n").expect("readme should be written");
    let out = run(&["doctor", bare.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&bare).expect("fixture should be swept");
    assert!(!out.contains("no guard wrapper"), "{out}");
    assert!(!out.contains("no ectropy.toml"), "{out}");
    assert!(out.contains("true to the skeleton"), "{out}");

    let seat = std::env::temp_dir().join("plumb-governed");
    std::fs::create_dir_all(seat.join(".runseal/wrappers")).expect("fixture should be made");
    let out = run(&["doctor", seat.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&seat).expect("fixture should be swept");
    assert!(out.contains("no guard wrapper"), "{out}");
    assert!(out.contains("no ectropy.toml"), "{out}");
}

#[test]
fn concurrency() {
    let dir = std::env::temp_dir().join("plumb-concurrency");
    std::fs::create_dir_all(dir.join(".runseal/wrappers")).expect("fixture should be made");
    std::fs::create_dir_all(dir.join(".forgejo/workflows")).expect("fixture should be made");
    for name in ["guard", "init", "land"] {
        std::fs::write(dir.join(format!(".runseal/wrappers/{name}.ts")), "")
            .expect("wrapper should be written");
    }
    std::fs::write(dir.join("ectropy.toml"), "").expect("laws should be written");
    let lane = dir.join(".forgejo/workflows/guard.yml");

    std::fs::write(
        &lane,
        "name: guard\non:\n  pull_request:\n\njobs:\n  guard: {}\n",
    )
    .expect("lane should be written");
    let bare = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(
        bare.contains("guard lane without the concurrency block"),
        "{bare}"
    );

    let complete = "name: guard\non:\n  pull_request:\n\nconcurrency:\n  group: guard-${{ github.event.pull_request.number || github.ref }}\n  cancel-in-progress: true\n\njobs:\n  guard: {}\n";
    std::fs::write(&lane, complete).expect("lane should be written");
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::write(&lane, complete.replace('\n', "\r\n")).expect("lane should be written");
    let windows = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!held.contains("concurrency block"), "{held}");
    assert!(!windows.contains("concurrency block"), "{windows}");
}

#[test]
fn edition() {
    let dir = std::env::temp_dir().join("plumb-edition");
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    let cargo = dir.join("Cargo.toml");

    std::fs::write(&cargo, "[workspace.package]\nedition = \"2021\"\n")
        .expect("manifest should be written");
    let old = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(
        old.contains("edition is 2021, the skeleton holds 2024"),
        "{old}"
    );

    std::fs::write(&cargo, "[workspace.package]\nedition = \"2024\"\n")
        .expect("manifest should be written");
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!held.contains("edition is"), "{held}");
}

#[test]
fn container() {
    let dir = std::env::temp_dir().join("plumb-container");
    std::fs::create_dir_all(dir.join(".forgejo/workflows")).expect("fixture should be made");
    let lane = dir.join(".forgejo/workflows/guard.yml");

    std::fs::write(&lane, "container: mirror.perish.lan/ci/deno:20260716-abc\n")
        .expect("lane should be written");
    let pin = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(pin.contains("CI container pinned to a tag"), "{pin}");

    std::fs::write(&lane, "container: mirror.perish.lan/ci/deno\n")
        .expect("lane should be written");
    let bare = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!bare.contains("CI container pinned"), "{bare}");
}

#[test]
fn clap() {
    let dir = std::env::temp_dir().join("plumb-clap");
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    std::fs::write(dir.join(".gitignore"), "target/\n").expect("ignore should be written");
    let cargo = dir.join("Cargo.toml");

    std::fs::write(
        &cargo,
        "[[bin]]\nname = \"x\"\n[dependencies]\ntoml = \"0.9\"\n",
    )
    .expect("manifest should be written");
    let bare = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(bare.contains("ships a rust binary without clap"), "{bare}");

    std::fs::write(
        &cargo,
        "[[bin]]\nname = \"x\"\n[dependencies]\nclap = \"4\"\n",
    )
    .expect("manifest should be written");
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);

    std::fs::write(&cargo, "[dependencies]\ntoml = \"0.9\"\n").expect("manifest should be written");
    let lib = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!held.contains("without clap"), "{held}");
    assert!(!lib.contains("without clap"), "{lib}");
}

#[test]
fn substrate() {
    let dir = std::env::temp_dir().join("plumb-substrate");
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    std::fs::write(dir.join(".gitignore"), "target/\n").expect("ignore should be written");
    let cargo = dir.join("Cargo.toml");

    std::fs::write(
        &cargo,
        "[[bin]]\nname = \"x\"\n[dependencies]\nclap = \"4\"\n",
    )
    .expect("manifest should be written");
    let bare = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(bare.contains("ships a rust binary without plumb"), "{bare}");

    std::fs::write(
        dir.join("Cargo.lock"),
        "[[package]]\nname = \"plumb\"\nversion = \"0.2.0\"\n",
    )
    .expect("lock should be written");
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!held.contains("without plumb"), "{held}");
}

#[test]
fn retired() {
    let dir = std::env::temp_dir().join("plumb-retired");
    std::fs::create_dir_all(dir.join(".runseal")).expect("fixture should be made");
    let map = dir.join(".runseal/deno.json");

    std::fs::write(
        &map,
        "{\"imports\":{\"@perish/harness\":\"jsr:@perish/harness@0.4.0\"}}",
    )
    .expect("map should be written");
    let old = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(
        old.contains("depends on @perish/harness, renamed to @perish/sealkit"),
        "{old}"
    );

    std::fs::write(
        &map,
        "{\"imports\":{\"@perish/sealkit\":\"jsr:@perish/sealkit\"}}",
    )
    .expect("map should be written");
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!held.contains("renamed to"), "{held}");
}

#[test]
fn pinned() {
    let dir = std::env::temp_dir().join("plumb-pinned");
    std::fs::create_dir_all(dir.join(".runseal")).expect("fixture should be made");
    let map = dir.join(".runseal/deno.json");

    std::fs::write(
        &map,
        "{\"imports\":{\"@perish/shield\":\"jsr:@perish/shield@0.1.0\"}}",
    )
    .expect("map should be written");
    let pin = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(
        pin.contains("self-built @perish/shield is version-pinned"),
        "{pin}"
    );

    std::fs::write(
        &map,
        "{\"imports\":{\"@perish/shield\":\"jsr:@perish/shield\",\"@std/cli\":\"jsr:@std/cli@1.0.0\"}}",
    )
    .expect("map should be written");
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!held.contains("version-pinned"), "{held}");
}

#[test]
fn packages() {
    let dir = std::env::temp_dir().join("plumb-packages");
    std::fs::create_dir_all(&dir).expect("fixture should be made");

    std::fs::write(
        dir.join("deno.json"),
        "{\"name\":\"@perish/foo\",\"exports\":\"./mod.ts\"}",
    )
    .expect("manifest should be written");
    let root = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(
        root.contains("publishable package @perish/foo at the root"),
        "{root}"
    );
    std::fs::remove_file(dir.join("deno.json")).expect("manifest should be swept");

    std::fs::create_dir_all(dir.join("packages/bar")).expect("fixture should be made");
    std::fs::write(
        dir.join("packages/bar/deno.json"),
        "{\"name\":\"@perish/foo\"}",
    )
    .expect("manifest should be written");
    let wrong = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(
        wrong.contains("package @perish/foo sits in packages/bar"),
        "{wrong}"
    );

    std::fs::create_dir_all(dir.join("packages/foo")).expect("fixture should be made");
    std::fs::write(
        dir.join("packages/foo/deno.json"),
        "{\"name\":\"@perish/foo\"}",
    )
    .expect("manifest should be written");
    std::fs::remove_dir_all(dir.join("packages/bar")).expect("fixture should be swept");
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(!held.contains("must match the name"), "{held}");

    std::fs::create_dir_all(dir.join("packages/components")).expect("fixture should be made");
    let reserved = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(
        reserved.contains("packages/components is reserved"),
        "{reserved}"
    );
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn hooks() {
    let dir = std::env::temp_dir().join("plumb-hookless");
    std::fs::create_dir_all(dir.join(".runseal/wrappers")).expect("fixture should be made");
    let bare = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(bare.contains("no pre-commit hook"), "{bare}");
    assert!(bare.contains("no commit-msg hook"), "{bare}");

    std::fs::create_dir_all(dir.join(".runseal/hooks")).expect("fixture should be made");
    for name in ["pre-commit", "commit-msg"] {
        std::fs::write(dir.join(format!(".runseal/hooks/{name}")), "#!/bin/sh\n")
            .expect("hook should be written");
    }
    let held = run(&["doctor", dir.to_str().expect("path should be utf8")]);
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
    assert!(!held.contains("no pre-commit hook"), "{held}");
    assert!(!held.contains("no commit-msg hook"), "{held}");
}
