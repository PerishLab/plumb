mod admission;
mod fixture;

use fixture::{Fixture, action, rejected, success};

#[test]
fn protection() {
    let fixture = Fixture::new();
    let manual = success(fixture.run());
    assert!(
        manual["actions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|action| action["name"] != "guard/lock")
    );
    fixture.policy(false, true);
    let seeded = success(fixture.run());
    assert!(
        seeded["actions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|action| action["name"] != "guard/lock")
    );
    let baseline = fixture.calls();
    fixture.policy(true, false);
    rejected(fixture.run(), "requires both guard/rust and guard/ectropy");
    fixture.policy(true, true);
    fixture.put("ectropy.toml", "");
    fixture.git(&["add", "."]);
    rejected(fixture.run(), "explicit Ectropy lock grant/ban");
    fixture.put(
        "ectropy.toml",
        "[[grant]]\nsyntax=\"lock\"\npaths=[\"src\"]\n",
    );
    fixture.git(&["add", "."]);
    rejected(fixture.run(), "regular file");
    fixture.put(
        "ectropy.toml",
        "[[ban]]\nsyntax=\"lock\"\npaths=[\"src/**\"]\n",
    );
    fixture.git(&["add", "."]);
    let first = success(fixture.run());
    assert_eq!(action(&first, "guard/lock")["name"], "guard/lock");
    assert_eq!(fixture.calls(), baseline + 1);
    let cached = success(fixture.run());
    assert_eq!(first, cached);
    assert_eq!(
        fixture.calls(),
        baseline + 1,
        "exact proof reuse must not rerun the syntax tool"
    );
    fixture.put("outside.txt", "changed outside the cropped workflow hash\n");
    fixture.git(&["add", "."]);
    let changed = success(fixture.run());
    assert_eq!(action(&first, "guard/rust"), action(&changed, "guard/rust"));
    for name in ["guard/lock", "guard/ectropy"] {
        assert_ne!(action(&first, name), action(&changed, name));
    }
    assert_eq!(fixture.calls(), baseline + 2);
    let restored = success(fixture.run());
    assert_eq!(changed, restored);
    assert_eq!(fixture.calls(), baseline + 2);
    std::fs::write(fixture.home.path().join("fail"), "fail").unwrap();
    fixture.put("outside.txt", "fresh input must observe syntax failure\n");
    fixture.git(&["add", "."]);
    rejected(fixture.run(), "ectropy . failed");
    std::fs::remove_file(fixture.home.path().join("fail")).unwrap();
    success(fixture.run());
    assert_eq!(
        fixture.calls(),
        baseline + 4,
        "a rejected syntax action must not be recorded as success"
    );
    fixture.put(
        "src/lib.rs",
        "pub fn native(file: &std::fs::File) {\n    let _ = file.try_lock();\n}\n",
    );
    fixture.git(&["add", "."]);
    rejected(fixture.run(), "lock disposition mismatch");
}

#[test]
fn evidence() {
    let fixture = Fixture::new();
    fixture.policy(true, true);
    fixture.put(
        "src/lib.rs",
        "pub fn native(file: &std::fs::File) {\n    let _ = file.try_lock();\n}\n",
    );
    fixture.put("protocol.txt", "original protocol\n");
    fixture.git(&["add", "."]);
    fixture.reviews();
    let first = success(fixture.run());
    let calls = fixture.calls();
    fixture.put("protocol.txt", "changed protocol\n");
    assert_eq!(
        success(fixture.run()),
        first,
        "unstaged evidence cannot change the staged proof"
    );
    fixture.git(&["add", "."]);
    rejected(fixture.run(), "evidence is stale");
    assert_eq!(
        fixture.calls(),
        calls,
        "stale evidence must refuse before any cached success or action execution"
    );
    fixture.put("protocol.txt", "original protocol\n");
    fixture.git(&["add", "."]);
    assert_eq!(success(fixture.run()), first);
}
