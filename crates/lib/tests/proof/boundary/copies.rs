use super::{Repo, inspect, run, text};

#[test]
fn destination() {
    let repo = Repo::new();
    repo.write("source/item.rs", "same\n");
    let base = repo.commit("base");
    repo.write("copied/item.rs", "same\n");
    let head = repo.commit("copy");
    let delta = text(run(
        repo.root(),
        &[
            "diff",
            "--name-status",
            "--find-copies-harder",
            &base,
            &head,
        ],
    ));
    assert_eq!(delta, "C100\tsource/item.rs\tcopied/item.rs");
    let report = inspect(&repo, &base, &head, &["source"]);
    assert_eq!(report.changed, ["copied/item.rs"]);
    assert_eq!(report.outside, ["copied/item.rs"]);
    assert!(!report.ok);
}

#[test]
fn modified() {
    let repo = Repo::new();
    repo.write("source/item.rs", "one\ntwo\nthree\nfour\nfive\n");
    let base = repo.commit("base");
    repo.write("copied/item.rs", "one\ntwo\nthree\nfour\nfive\n");
    repo.write("source/item.rs", "one\ntwo\nthree\nfour\nchanged\n");
    let head = repo.commit("copy and modify");
    let delta = text(run(
        repo.root(),
        &[
            "diff",
            "--name-status",
            "--find-copies-harder",
            &base,
            &head,
        ],
    ));
    assert!(delta.contains("C100\tsource/item.rs\tcopied/item.rs"));
    assert!(delta.contains("M\tsource/item.rs"));
    let report = inspect(&repo, &base, &head, &["copied"]);
    assert_eq!(report.changed, ["copied/item.rs", "source/item.rs"]);
    assert_eq!(report.outside, ["source/item.rs"]);
    assert!(!report.ok);
}

#[test]
fn renamed() {
    let repo = Repo::new();
    repo.write("source/item.rs", "same\n");
    let base = repo.commit("base");
    repo.write("copied/first.rs", "same\n");
    repo.write("copied/second.rs", "same\n");
    std::fs::remove_file(repo.root().join("source/item.rs")).expect("delete origin");
    let head = repo.commit("copy and delete");
    let report = inspect(&repo, &base, &head, &["copied"]);
    assert_eq!(
        report.changed,
        ["copied/first.rs", "copied/second.rs", "source/item.rs"]
    );
    assert_eq!(report.outside, ["source/item.rs"]);
    assert!(!report.ok);
}
