use super::{command, marked};
use crate::world::{Court, serve};

#[test]
fn terminal() {
    let fixture = tempfile::tempdir().expect("fixture");
    let bare = tempfile::tempdir().expect("bare");
    let (url, calls) = serve(Court::Expanding(std::sync::atomic::AtomicUsize::new(0)), 10);
    marked(fixture.path(), bare.path(), &url, "v1.2.0-nightly.8");
    let output = command(
        fixture.path(),
        &[
            "ship",
            "dispatch",
            "--marker",
            "v1.2.0-nightly.8",
            "--watch",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = calls.lock().expect("calls");
    assert!(
        calls
            .iter()
            .filter(|call| call.contains("/actions/tasks?"))
            .count()
            >= 5,
        "watcher must survive a transient terminal run state"
    );
}
