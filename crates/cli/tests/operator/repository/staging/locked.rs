use super::source::seat;
use super::{Repo, cache, registered, support};
use std::path::Path;

fn listed(root: &Path, source: &Path) -> Option<String> {
    let listed = String::from_utf8(Repo::git(root, &["worktree", "list", "--porcelain"]).stdout)
        .expect("utf8");
    let source = source.canonicalize().expect("source");
    listed
        .split("\n\n")
        .find(|entry| {
            entry
                .lines()
                .next()
                .and_then(|line| line.strip_prefix("worktree "))
                .is_some_and(|path| Path::new(path).canonicalize().ok().as_ref() == Some(&source))
        })
        .and_then(|entry| {
            entry
                .lines()
                .find_map(|line| line.strip_prefix("locked"))
                .map(|reason| reason.trim().to_owned())
        })
}

fn age(source: &Path) {
    std::fs::File::open(source.with_extension("lock"))
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(7200))
        .unwrap();
}

#[test]
fn reason() {
    let fixture = cache::fixture();
    let home = support::home();
    cache::success(&cache::run(fixture.path(), home.path()));
    let source = seat(home.path());
    assert_eq!(
        listed(fixture.path(), &source).as_deref(),
        Some(plumb::integration::staging::REASON)
    );
    assert!(plumb::integration::staging::REASON.starts_with("plumb guard source seat"));
}

#[test]
fn reuse() {
    let fixture = cache::fixture();
    let home = support::home();
    cache::success(&cache::run(fixture.path(), home.path()));
    let source = seat(home.path());
    Repo::git(
        fixture.path(),
        &["worktree", "unlock", source.to_str().unwrap()],
    );
    assert_eq!(listed(fixture.path(), &source), None);
    cache::success(&cache::run(fixture.path(), home.path()));
    assert_eq!(seat(home.path()), source);
    assert_eq!(
        listed(fixture.path(), &source).as_deref(),
        Some(plumb::integration::staging::REASON)
    );
}

#[test]
fn missing() {
    let fixture = cache::fixture();
    let home = support::home();
    cache::success(&cache::run(fixture.path(), home.path()));
    let source = seat(home.path());
    std::fs::remove_dir_all(&source).unwrap();
    cache::success(&cache::run(fixture.path(), home.path()));
    assert!(source.join("src/lib.rs").is_file());
    assert_eq!(registered(fixture.path()).len(), 1);
    assert_eq!(
        listed(fixture.path(), &source).as_deref(),
        Some(plumb::integration::staging::REASON)
    );
}

#[test]
fn idle() {
    let fixture = cache::fixture();
    let home = support::home();
    cache::success(&cache::run(fixture.path(), home.path()));
    let source = seat(home.path());
    plumb::integration::staging::reap(&home.path().join("tmp"));
    assert!(source.join("src/lib.rs").is_file());
    age(&source);
    plumb::integration::staging::reap(&home.path().join("tmp"));
    assert!(!source.exists());
    assert!(!source.with_extension("lock").exists());
    assert!(registered(fixture.path()).is_empty());
}

#[test]
fn orphan() {
    let fixture = cache::fixture();
    let home = support::home();
    cache::success(&cache::run(fixture.path(), home.path()));
    let source = seat(home.path());
    assert!(listed(fixture.path(), &source).is_some());
    drop(fixture);
    age(&source);
    plumb::integration::staging::reap(&home.path().join("tmp"));
    assert!(!source.exists());
    assert!(!source.with_extension("lock").exists());
}
