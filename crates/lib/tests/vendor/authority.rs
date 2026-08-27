#[test]
fn scopes() {
    assert_eq!(
        plumb::forgejo::scopes().into_iter().collect::<Vec<_>>(),
        ["public-only", "read:user", "write:package"]
    );
}
