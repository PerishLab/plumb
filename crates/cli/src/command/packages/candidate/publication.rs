use super::{Fixture, work};
use serde_json::json;

#[test]
fn unchanged() {
    let fixture = Fixture::new(json!({}));
    let root = fixture.repository("pub fn answer() -> u8 { 42 }\n");
    let origin = fixture.root.path().join("origin.git");
    work::git(&origin, &["config", "core.logAllRefUpdates", "true"]).unwrap();
    work::git(&root, &["checkout", "-q", "-b", "auto/17"]).unwrap();
    std::fs::write(
        root.join("Cargo.lock"),
        "version = 4\n# publication fixture\n",
    )
    .unwrap();
    work::git(&root, &["add", "Cargo.lock"]).unwrap();
    work::git(&root, &["commit", "-q", "-m", "Refs #17."]).unwrap();
    let head = work::git(&root, &["rev-parse", "HEAD"]).unwrap();
    work::git(&root, &["push", "-q", "origin", "auto/17"]).unwrap();
    let before = work::git(
        &origin,
        &["reflog", "show", "--format=%H", "refs/heads/auto/17"],
    )
    .unwrap();
    for _ in 0..2 {
        fixture
            .provider()
            .publish(
                17,
                super::super::super::pull::Publication {
                    root: &root,
                    head: &head,
                    previous: Some(&head),
                },
            )
            .unwrap();
        assert_eq!(work::git(&root, &["rev-parse", "HEAD"]).unwrap(), head);
        assert_eq!(
            work::git(&origin, &["rev-parse", "refs/heads/auto/17"]).unwrap(),
            head
        );
        assert_eq!(
            work::git(
                &origin,
                &["reflog", "show", "--format=%H", "refs/heads/auto/17"]
            )
            .unwrap(),
            before
        );
    }
    assert!(!fixture.calls().iter().any(|call| {
        call["payload"]["body"]
            .as_str()
            .is_some_and(|body| body.contains("plumb.auto-push/"))
    }));
}
