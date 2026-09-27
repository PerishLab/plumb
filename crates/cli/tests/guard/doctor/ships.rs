#[test]
fn cargo() {
    let dir = std::env::temp_dir().join("plumb-crate-seat");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    crate::govern(&dir);
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release]\nproduct = \"foo\"\nauthority = \"https://example.invalid\"\nbinaries = [\"foo\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n",
    )
    .expect("manifest should be written");
    let binary = crate::run(&["doctor", path]);
    assert!(binary.contains("publishes binary"), "{binary}");
    assert!(!binary.contains("cargo"), "{binary}");

    assert!(!dir.join(".forgejo").exists());

    std::fs::write(
        dir.join("plumb.toml"),
        "[release.cargo]\nregistry = \"perish\"\npackages = [\"foo\"]\n",
    )
    .expect("manifest should be written");
    let crates = crate::run(&["doctor", path]);
    assert!(crates.contains("publishes cargo"), "{crates}");
    assert!(!crates.contains("publishes binary"), "{crates}");
    assert!(!crates.contains("release wrapper"), "{crates}");

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn packages() {
    let dir = std::env::temp_dir().join("plumb-packages");
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    crate::govern(&dir);

    std::fs::write(
        dir.join("deno.json"),
        "{\"name\":\"@perish/foo\",\"exports\":\"./mod.ts\"}",
    )
    .expect("manifest should be written");
    let root = crate::run(&["doctor", dir.to_str().expect("path should be utf8")]);
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
    let wrong = crate::run(&["doctor", dir.to_str().expect("path should be utf8")]);
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
    let held = crate::run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(!held.contains("must match the name"), "{held}");

    std::fs::create_dir_all(dir.join("packages/components")).expect("fixture should be made");
    let reserved = crate::run(&["doctor", dir.to_str().expect("path should be utf8")]);
    assert!(
        reserved.contains("packages/components is reserved"),
        "{reserved}"
    );
    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
}

#[test]
fn placed() {
    let fixture = tempfile::tempdir().expect("fixture");
    let dir = fixture.path();
    crate::govern(dir);
    let path = dir.to_str().expect("path should be utf8");
    let units = dir.join("packaging/deb/root/lib/systemd/system");
    std::fs::create_dir_all(&units).expect("unit seat");
    std::fs::write(
        dir.join("packaging/deb/control"),
        "Package: foo-api\nVersion: __VERSION__\n",
    )
    .expect("control");
    std::fs::write(
        units.join("foo-api.service"),
        "[Service]\nExecStart=/usr/bin/foo-api\n",
    )
    .expect("unit");
    let manifest = "[release]\nproduct = \"foo\"\nauthority = \"https://example.invalid\"\nbinaries = [\"foo\", \"foo-api\"]\ntargets = [\"x86_64-unknown-linux-gnu\", \"aarch64-apple-darwin\"]\n[release.binary.foo-api]\ntargets = [\"x86_64-unknown-linux-gnu\"]\ninstall = false\n[release.deb]\nbinary = \"foo-api\"\nroot = \"packaging/deb\"\n";
    std::fs::write(dir.join("plumb.toml"), manifest).expect("manifest");
    let held = crate::run(&["doctor", path]);
    assert!(held.contains("publishes binary deb"), "{held}");
    assert!(
        held.contains("binaries  foo, foo-api on x86_64-unknown-linux-gnu uninstalled"),
        "{held}"
    );
    assert!(held.contains("places    deb foo-api"), "{held}");
    assert!(!held.contains("the current Plumb refuses"), "{held}");

    std::fs::write(
        dir.join("plumb.toml"),
        manifest.replace("binary = \"foo-api\"", "binary = \"foo\""),
    )
    .expect("manifest");
    let captured = crate::run(&["doctor", "--json", path]);
    let report: serde_json::Value = serde_json::from_str(&captured).expect("doctor json");
    assert!(
        report["findings"]
            .as_array()
            .is_some_and(|findings| findings.iter().any(|finding| {
                finding["code"] == "release.spec-declared"
                    && finding["evidence"]
                        .as_str()
                        .is_some_and(|evidence| evidence.contains("Package is foo-api"))
            })),
        "{captured}"
    );
}
