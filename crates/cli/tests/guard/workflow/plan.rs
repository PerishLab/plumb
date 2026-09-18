use super::{PAIR, seat};

#[test]
fn ambiguity() {
    let root = seat("world");
    root.declared(PAIR);
    root.git(&["commit", "-m", "base"]);
    let (text, ok) = root.plan(Some("HEAD"), &["runner=one", "runner=two"]);
    assert!(!ok, "{text}");
}
