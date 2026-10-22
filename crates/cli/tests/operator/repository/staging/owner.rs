use super::source::seat;
use super::{Repo, cache, support};

#[test]
fn missing() {
    let fixture = cache::fixture();
    let home = support::home();
    cache::success(&cache::run(fixture.path(), home.path()));
    let source = seat(home.path());
    let record = source.with_extension("lock");
    std::fs::remove_file(&record).unwrap();
    let refused = cache::run(fixture.path(), home.path());
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("without its owner record"));
    assert!(source.join("src/lib.rs").is_file());
    assert!(!record.exists());
}

#[test]
fn foreign() {
    let fixture = cache::fixture();
    let other = cache::fixture();
    let home = support::home();
    cache::success(&cache::run(fixture.path(), home.path()));
    let source = seat(home.path());
    Repo::git(
        fixture.path(),
        &[
            "worktree",
            "remove",
            "--force",
            "--force",
            source.to_str().unwrap(),
        ],
    );
    Repo::git(other.path(), &["commit", "-qm", "foreign source"]);
    Repo::git(
        other.path(),
        &[
            "worktree",
            "add",
            "--detach",
            "--quiet",
            source.to_str().unwrap(),
        ],
    );
    std::fs::write(source.join("held"), "foreign content").unwrap();
    let refused = cache::run(fixture.path(), home.path());
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("another Git repository"));
    assert_eq!(
        std::fs::read_to_string(source.join("held")).unwrap(),
        "foreign content"
    );
    plumb::integration::staging::reap(&home.path().join("tmp"));
    assert!(source.join("held").is_file());
}

#[test]
fn dirty() {
    let fixture = cache::fixture();
    let home = support::home();
    cache::success(&cache::run(fixture.path(), home.path()));
    let source = seat(home.path());
    std::fs::write(source.join("src/lib.rs"), "unverified changes\n").unwrap();
    let record = std::fs::File::open(source.with_extension("lock")).unwrap();
    record
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(7200))
        .unwrap();
    plumb::integration::staging::reap(&home.path().join("tmp"));
    let refused = cache::run(fixture.path(), home.path());
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("unverified tracked changes"));
    assert_eq!(
        std::fs::read_to_string(source.join("src/lib.rs")).unwrap(),
        "unverified changes\n"
    );
}

#[test]
fn alias() {
    let fixture = cache::fixture();
    let home = support::home();
    cache::success(&cache::run(fixture.path(), home.path()));
    let source = seat(home.path());
    Repo::git(
        fixture.path(),
        &[
            "worktree",
            "remove",
            "--force",
            "--force",
            source.to_str().unwrap(),
        ],
    );
    let other = tempfile::tempdir().unwrap();
    std::fs::write(other.path().join("held"), "foreign content").unwrap();
    std::os::unix::fs::symlink(other.path(), &source).unwrap();
    let refused = cache::run(fixture.path(), home.path());
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("not an owned directory"));
    assert_eq!(
        std::fs::read_to_string(other.path().join("held")).unwrap(),
        "foreign content"
    );
}

#[test]
fn gone() {
    let fixture = cache::fixture();
    let home = support::home();
    cache::success(&cache::run(fixture.path(), home.path()));
    let source = seat(home.path());
    let record = source.with_extension("lock");
    drop(fixture);
    plumb::integration::staging::reap(&home.path().join("tmp"));
    assert!(source.join("src/lib.rs").is_file());
    std::fs::File::open(&record)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(7200))
        .unwrap();
    plumb::integration::staging::reap(&home.path().join("tmp"));
    assert!(!source.exists());
    assert!(!record.exists());
}
