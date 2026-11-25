#[test]
fn multi() {
    let (_scratch, dir) = super::seat("plumb-release-multi-oci");
    let path = dir.to_str().expect("path should be utf8");
    std::fs::create_dir_all(dir.join("apps/web")).expect("web dir");
    std::fs::create_dir_all(dir.join("deploy")).expect("deploy dir");
    std::fs::write(
        dir.join("apps/web/package.json"),
        r#"{"name":"foo-web","scripts":{"build":"vite build"}}"#,
    )
    .expect("package");
    std::fs::write(
        dir.join("deploy/web.Dockerfile"),
        "FROM node:22\nCMD [\"node\", \"dist/.perish/server.mjs\"]\n",
    )
    .expect("dockerfile");
    std::fs::write(
        dir.join("plumb.toml"),
        "[identity]\nns = \"foo\"\n[release]\nauthority = \"https://example.invalid\"\nbinaries = [\"foo\", \"foo-api\"]\ntargets = [\"x86_64-unknown-linux-gnu\", \"aarch64-apple-darwin\"]\n[release.binary.foo-api]\ntargets = [\"x86_64-unknown-linux-gnu\"]\ninstall = false\n[release.oci]\nregistry = \"ghcr.io\"\naccount = \"PerishLab\"\n[release.oci.components.api]\nimage = \"perishlab/foo-api\"\nbinary = \"foo-api\"\n[release.oci.components.web]\nimage = \"perishlab/foo-web\"\nweb = \"apps/web\"\nfile = \"deploy/web.Dockerfile\"\n",
    )
    .expect("manifest");
    let held = crate::run(&["doctor", path]);
    assert!(!held.contains("the current Plumb refuses"), "{held}");
    assert!(held.contains("publishes binary oci"), "{held}");
    assert!(
        held.contains("places    oci api foo-api, oci web apps/web"),
        "{held}"
    );
}
