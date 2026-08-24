use super::seat;

const RELEASE: &str = "[release]\n\
product = \"seat\"\n\
authority = \"https://releases.example.com\"\n\
binaries = [\"seat\"]\n\
targets = [\"x86_64-unknown-linux-gnu\"]\n";

#[test]
fn plain() {
    let root = seat("plain");
    root.declared(RELEASE);
    let text = root.rendered();
    assert!(text.contains("cargo fmt --all --check"), "{text}");
    assert!(!text.contains("plumb workflow ask"), "{text}");
    assert!(!text.contains("steps.workflow.outputs"), "{text}");
    assert!(!text.contains("PLUMB_LOCK_ACCESS"), "{text}");
}

#[test]
fn guarded() {
    let root = seat("guarded");
    root.declared(&format!(
        "{RELEASE}\n[workflow.hash.guard]\n\"rust\" = [\"suite://cargo\"]\n"
    ));
    let text = root.rendered();
    assert!(text.contains("plumb workflow ask guard"), "{text}");
    assert!(text.contains("- name: Guard rust"), "{text}");
    assert!(
        text.contains("if: steps.workflow.outputs.rust != 'false'"),
        "{text}"
    );
    assert!(
        text.contains("plumb workflow lock guard/rust || true"),
        "{text}"
    );
    assert!(!text.contains("steps.workflow.outputs.test"), "{text}");
    assert!(
        text.contains("PLUMB_LOCK_ACCESS: ${{ secrets.workflow_lock_s3_access_key }}"),
        "{text}"
    );
    assert!(
        text.contains("PLUMB_WORKFLOW_SEAT: ${{ github.repository }}"),
        "{text}"
    );
}
