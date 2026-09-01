use super::{command, marked};
use crate::world::{Court, serve};

#[test]
fn timeout() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let (url, _calls) = serve(Court::Flight, 4000);
    marked(fixture.path(), bare.path(), &url, "v1.2.0-nightly.1");
    let output = command(
        fixture.path(),
        &[
            "ship",
            "dispatch",
            "--marker",
            "v1.2.0-nightly.1",
            "--watch",
        ],
    );
    assert!(!output.status.success());
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(
        text.contains("still running past the watch timeout"),
        "{text}"
    );
}
