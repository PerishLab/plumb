use std::process::Command;

fn surface(root: &std::path::Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["release", "surface"])
        .env("PLUMB_RELEASE_ROOT", root)
        .output()
        .expect("plumb should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn declared() {
    let root = std::env::temp_dir().join("plumb-release-surface");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("skills/foo")).expect("fixture");

    std::fs::write(
        root.join("plumb.toml"),
        "[release.cargo]\nregistry = \"perish\"\npackages = [\"foo\"]\n",
    )
    .expect("manifest");
    assert_eq!(
        surface(&root),
        r#"{"include":[{"medium":"cargo"}],"project":{"include":[{"medium":"cargo"}]},"seal":{"include":[]}}"#
    );

    std::fs::write(
        root.join("plumb.toml"),
        "[release]\nproduct = \"foo\"\nauthority = \"https://example.invalid\"\nbinaries = [\"foo\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\nskill = true\n[release.oci]\nregistry = \"example.invalid\"\nimage = \"owner/foo\"\naccount = \"Example\"\n",
    )
    .expect("manifest");
    assert_eq!(
        surface(&root),
        r#"{"include":[{"medium":"binary"},{"medium":"oci"}],"project":{"include":[{"medium":"oci"}]},"seal":{"include":[{"held":"seal"}]}}"#
    );

    std::fs::remove_dir_all(&root).expect("fixture should be swept");
}

fn refusal(root: &std::path::Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_plumb"))
        .args(["release", "surface"])
        .env("PLUMB_RELEASE_ROOT", root)
        .output()
        .expect("plumb should run");
    assert!(!output.status.success());
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

#[test]
fn worker() {
    let root = std::env::temp_dir().join("plumb-release-cfworker");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("apps")).expect("fixture");
    let manifest = root.join("plumb.toml");

    std::fs::write(
        &manifest,
        "[release.cfworker]\naccount = \"held\"\ndomain = \"probe.example.uk\"\n",
    )
    .expect("manifest");
    assert_eq!(
        surface(&root),
        r#"{"include":[{"medium":"cfworker"}],"project":{"include":[{"medium":"cfworker"}]},"seal":{"include":[]}}"#
    );

    std::fs::write(
        &manifest,
        "[release.cfworker]\naccount = \"held\"\ndomain = \"probe_one.example.uk\"\n",
    )
    .expect("manifest");
    assert!(
        refusal(&root).contains("RFC 1123"),
        "an underscore is not a hostname label"
    );

    std::fs::write(
        &manifest,
        "[release.cfworker]\naccount = \"held\"\ndomain = \"probe.example.uk\"\npreview = \"held.example.uk\"\n",
    )
    .expect("manifest");
    assert!(
        refusal(&root).contains("unknown field"),
        "a preview root is derived, never declared"
    );

    std::fs::remove_dir_all(&root).expect("fixture should be swept");
}

#[test]
fn depends() {
    let root = std::env::temp_dir().join("plumb-release-depends");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("packages/held")).expect("fixture");
    std::fs::create_dir_all(root.join("packages/bone")).expect("fixture");
    let manifest = root.join("plumb.toml");

    std::fs::write(
        &manifest,
        "[release.npm]\nregistry = \"https://example.invalid\"\npackages = [\"held\"]\n[release.depends]\n\"npm/held\" = [\"packages/bone\"]\n",
    )
    .expect("manifest");
    assert!(
        surface(&root).contains("npm"),
        "a declared extra root does not change the surface"
    );

    std::fs::write(
        &manifest,
        "[release.npm]\nregistry = \"https://example.invalid\"\npackages = [\"held\"]\n[release.depends]\n\"npm/absent\" = [\"packages/bone\"]\n",
    )
    .expect("manifest");
    assert!(
        surface(&root).contains("npm"),
        "the surface reads before objects resolve"
    );

    std::fs::remove_dir_all(&root).expect("fixture should be swept");
}
