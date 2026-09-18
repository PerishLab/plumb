use super::{Repo, cache};

#[test]
#[cfg(unix)]
fn ancestor() {
    let fixture = cache::fixture();
    let root = fixture.path();
    let home = super::seat();
    let physical = tempfile::tempdir().expect("physical seat");
    let target = physical.path().join("parent/seat");
    std::fs::create_dir_all(&target).expect("physical parent");
    std::os::unix::fs::symlink(&target, home.path().join("tmp")).expect("seat alias");
    cache::success(&cache::run(root, home.path()));
    let configuration = physical.path().join("parent/.cargo");
    std::fs::create_dir(&configuration).expect("host configuration");
    std::fs::write(
        configuration.join("config.toml"),
        "[build]\nrustflags = ['--invalid-probe']\n",
    )
    .expect("host override");
    for changed in [false, true] {
        if changed {
            std::fs::write(root.join("NOTES"), "unrelated\n").expect("unrelated");
            Repo::git(root, &["add", "NOTES"]);
        }
        let output = cache::run(root, home.path());
        assert!(
            !output.status.success(),
            "physical ancestor must not reuse proof"
        );
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("unbound host configuration"), "{error}");
        assert!(!error.contains("guard guard/rust"), "{error}");
    }
}

#[test]
#[cfg(unix)]
fn dangling() {
    let fixture = cache::fixture();
    let home = super::seat();
    let physical = tempfile::tempdir().expect("physical seat");
    std::os::unix::fs::symlink(physical.path().join("absent"), home.path().join("tmp"))
        .expect("broken alias");
    let output = cache::run(fixture.path(), home.path());
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("cannot resolve Cargo execution path"),
        "{error}"
    );
    assert!(!error.contains("guard guard/rust"), "{error}");
}
