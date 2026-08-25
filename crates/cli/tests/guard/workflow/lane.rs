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
    assert!(
        text.contains("uses: PerishLab/actions/.forgejo/workflows/guard.atom.yml@main"),
        "{text}"
    );
    assert!(!text.contains("cargo fmt --all --check"), "{text}");
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
    assert!(text.contains("guard.atom.yml@main"), "{text}");
    assert!(!text.contains("plumb workflow ask guard"), "{text}");
    assert!(!text.contains("PLUMB_LOCK_ACCESS"), "{text}");
}

#[test]
fn bootstrap() {
    let root = seat("bootstrap");
    root.declared(&RELEASE.replace("product = \"seat\"", "product = \"plumb\""));
    let text = root.rendered();
    assert!(text.contains("cargo fmt --all --check"), "{text}");
    assert!(
        text.contains("cargo run --quiet --locked --bin seat -- doctor ."),
        "{text}"
    );
    assert!(!text.contains("guard.atom.yml@main"), "{text}");
}

#[test]
fn local() {
    let root = seat("owns-atom-locally");
    root.wrote(
        ".forgejo/workflows/guard.atom.yml",
        "name: guard atom\non:\n  workflow_call:\n",
    );
    root.declared(RELEASE);
    let text = root.rendered();
    assert!(
        text.contains("uses: ./.forgejo/workflows/guard.atom.yml"),
        "{text}"
    );
    assert!(!text.contains("@main"), "{text}");
}
