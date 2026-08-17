const BINARY: &str = "[release]\nproduct = \"foo\"\nauthority = \"https://example.invalid\"\nbinaries = [\"foo\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n";

fn seat(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    crate::govern(&dir);
    dir
}

fn caller(dir: &std::path::Path, lane: &str, shared: &str) {
    std::fs::create_dir_all(dir.join(".forgejo/workflows")).expect("lanes should be made");
    std::fs::write(
        dir.join(format!(".forgejo/workflows/{lane}.yml")),
        format!(
            "jobs:\n  release:\n    uses: PerishLab/actions/.forgejo/workflows/{shared}.yml@main\n"
        ),
    )
    .expect("lane should be written");
}

#[test]
fn refused() {
    let dir = seat("plumb-release-declaration");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(dir.join("plumb.toml"), BINARY).expect("manifest should be written");
    let whole = crate::run(&["doctor", path]);
    assert!(!whole.contains("the current Plumb refuses"), "{whole}");
    assert!(whole.contains("publishes binary"), "{whole}");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release]\nproduct = \"foo\"\nauthority = \"https://example.invalid\"\nbinaries = [\"foo\"]\n[release.cargo]\nregistry = \"perish\"\npackages = [\"foo\"]\n\ntargets = [\"x86_64-unknown-linux-gnu\"]\n",
    )
    .expect("manifest should be written");
    let captured = crate::run(&["doctor", path]);
    assert!(captured.contains("the current Plumb refuses"), "{captured}");
    assert!(captured.contains("cannot parse"), "{captured}");

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn deliverable() {
    let dir = seat("plumb-release-attachment");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(dir.join("plumb.toml"), BINARY).expect("manifest should be written");
    let uncalled = crate::run(&["doctor", path]);
    assert!(
        uncalled.contains("binary attachment is declared and this repository calls none of"),
        "{uncalled}"
    );

    caller(&dir, "release-exact", "release-binary");
    caller(&dir, "release-stable", "release-binary");
    let called = crate::run(&["doctor", path]);
    assert!(
        !called.contains("binary attachment is declared"),
        "{called}"
    );

    std::fs::write(
        dir.join("plumb.toml"),
        format!(
            "{BINARY}\n[release.oci]\nregistry = \"example.invalid\"\nimage = \"perishlab/foo\"\naccount = \"Example\"\n"
        ),
    )
    .expect("manifest should be written");
    let held = crate::run(&["doctor", path]);
    assert!(held.contains("publishes binary oci"), "{held}");
    assert!(!held.contains("attachment is declared"), "{held}");

    for lane in ["release-exact", "release-stable"] {
        caller(&dir, lane, "release-cargo");
    }
    let wrong = crate::run(&["doctor", path]);
    assert!(
        wrong.contains("oci attachment is declared and this repository calls none of"),
        "{wrong}"
    );

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn carried() {
    let dir = seat("plumb-release-cargo-only");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release.cargo]\nregistry = \"perish\"\npackages = [\"foo\"]\n",
    )
    .expect("manifest should be written");
    caller(&dir, "release-exact", "release-cargo");
    let held = crate::run(&["doctor", path]);
    assert!(held.contains("publishes cargo"), "{held}");
    assert!(!held.contains("the current Plumb refuses"), "{held}");
    assert!(!held.contains("cargo attachment is declared"), "{held}");

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn attached() {
    let dir = seat("plumb-release-attachment-only");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release.oci]\nregistry = \"git.perish.top\"\nimage = \"owner/name\"\naccount = \"PerishFire\"\n",
    )
    .expect("manifest should be written");
    caller(&dir, "release-exact", "release-binary");
    let image = crate::run(&["doctor", path]);
    assert!(image.contains("publishes oci"), "{image}");
    assert!(!image.contains("the current Plumb refuses"), "{image}");

    std::fs::write(dir.join("plumb.toml"), "[release]\nskill = false\n")
        .expect("manifest should be written");
    let empty = crate::run(&["doctor", path]);
    assert!(empty.contains("the current Plumb refuses"), "{empty}");
    assert!(empty.contains("at least one attachment"), "{empty}");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release]\nskill = true\n[release.cargo]\nregistry = \"perish\"\npackages = [\"foo\"]\n",
    )
    .expect("manifest should be written");
    let skill = crate::run(&["doctor", path]);
    assert!(skill.contains("requires a binary release"), "{skill}");

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}
