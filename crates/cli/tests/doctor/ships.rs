#[test]
fn cargo() {
    let dir = std::env::temp_dir().join("plumb-crate-seat");
    std::fs::create_dir_all(&dir).expect("fixture should be made");
    let path = dir.to_str().expect("path should be utf8");

    std::fs::write(
        dir.join("plumb.toml"),
        "[release]\nproduct = \"foo\"\nauthority = \"https://example.invalid\"\nbinaries = [\"foo\"]\n",
    )
    .expect("manifest should be written");
    let binary = crate::run(&["doctor", path]);
    assert!(binary.contains("publishes binary"), "{binary}");
    assert!(!binary.contains("cargo"), "{binary}");

    std::fs::create_dir_all(dir.join(".forgejo/workflows")).expect("lanes should be made");
    for lane in ["release-exact", "release-stable"] {
        std::fs::write(
            dir.join(format!(".forgejo/workflows/{lane}.yml")),
            "jobs:\n  release:\n    with:\n      ref: ${{ inputs.ref }}\n",
        )
        .expect("lane should be written");
    }
    let stale = crate::run(&["doctor", path]);
    assert!(
        stale.contains("does not bind github.ref and github.sha"),
        "{stale}"
    );
    for lane in ["release-exact", "release-stable"] {
        std::fs::write(
            dir.join(format!(".forgejo/workflows/{lane}.yml")),
            "jobs:\n  release:\n    with:\n      source_ref: ${{ github.ref }}\n      source_commit: ${{ github.sha }}\n",
        )
        .expect("lane should be written");
    }
    let bound = crate::run(&["doctor", path]);
    assert!(
        !bound.contains("does not bind github.ref and github.sha"),
        "{bound}"
    );

    std::fs::write(
        dir.join("plumb.toml"),
        "[release.cargo]\nregistry = \"perish\"\npackages = [\"foo\"]\n",
    )
    .expect("manifest should be written");
    let crates = crate::run(&["doctor", path]);
    assert!(crates.contains("publishes cargo"), "{crates}");
    assert!(!crates.contains("publishes binary"), "{crates}");
    assert!(
        crates.contains("declares cargo without a release wrapper"),
        "{crates}"
    );

    std::fs::remove_dir_all(&dir).expect("fixture should be swept");
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
