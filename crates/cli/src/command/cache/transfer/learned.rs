use super::{Action, Transfer};

#[test]
fn history() {
    let root = tempfile::tempdir().unwrap();
    assert!(
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(root.path())
            .status()
            .unwrap()
            .success()
    );
    let producer = root.path().join("producer");
    std::fs::create_dir_all(producer.join("cache/compiler")).unwrap();
    std::fs::create_dir_all(producer.join("state/guard/test")).unwrap();
    std::fs::write(producer.join("cache/compiler/entry"), "compiled").unwrap();
    std::fs::write(producer.join("state/guard/test/held.json"), "incoming").unwrap();
    std::fs::write(producer.join("state/guard/test/new.json"), "learned").unwrap();
    let parcel = root.path().join("slot/material.tar.zst");
    std::fs::create_dir(parcel.parent().unwrap()).unwrap();
    let saved = Transfer {
        root: root.path(),
        home: &producer,
        context: None,
        parcel: &parcel,
        family: "world;",
        identity: "world;producer",
    }
    .run(Action::Save)
    .unwrap();
    assert!(saved.compiler);
    let consumer = root.path().join("consumer");
    std::fs::create_dir_all(consumer.join("state/guard/test")).unwrap();
    std::fs::write(consumer.join("state/guard/test/held.json"), "existing").unwrap();
    let restored = Transfer {
        root: root.path(),
        home: &consumer,
        context: None,
        parcel: &parcel,
        family: "world;",
        identity: "world;other",
    }
    .run(Action::Restore { fetch: false })
    .unwrap();
    assert!(restored.compiler);
    assert_eq!(
        std::fs::read_to_string(consumer.join("state/guard/test/held.json")).unwrap(),
        "existing"
    );
    assert_eq!(
        std::fs::read_to_string(consumer.join("state/guard/test/new.json")).unwrap(),
        "learned"
    );
}
