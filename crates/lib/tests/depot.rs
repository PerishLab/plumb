#[path = "support/generation.rs"]
mod generation;

#[test]
fn skill() {
    let root = tempfile::tempdir().expect("skill seat");
    std::fs::create_dir_all(root.path().join("home/.claude/skills")).expect("agent seat");
    let kit = plumb::skill::Kit {
        name: "probe".to_string(),
        home: root.path().join("home"),
        state: root.path().join("state.json"),
        url: "https://releases.example.test".to_string(),
    };
    let source = kit.depot("https://depot.example.test", "probe", "v1.2.3");
    let held = source.install(&plumb::skill::Ask {
        channel: "stable".to_string(),
        version: Some("v1.2.2".to_string()),
        ..plumb::skill::Ask::default()
    });
    assert!(matches!(held, Err(plumb::skill::Error::Version(_))));
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Note {
    name: String,
}

#[test]
fn record() {
    let root = tempfile::tempdir().expect("record seat");
    let path = root.path().join("deep/note.json");
    assert!(matches!(plumb::record::read::<Note>(&path, 3), Ok(None)));
    let note = Note {
        name: "probe".to_string(),
    };
    plumb::record::write(&path, 3, &note).expect("record written");
    let text = std::fs::read_to_string(&path).expect("record text");
    assert_eq!(text, "{\n  \"schema\": 3,\n  \"name\": \"probe\"\n}\n");
    let held = plumb::record::read::<Note>(&path, 3).expect("record read");
    assert_eq!(held, Some(note));
    let foreign = plumb::record::read::<Note>(&path, 4);
    assert!(matches!(foreign, Err(plumb::record::Error::Schema(_, 3))));
    std::fs::write(&path, "{\"name\": \"probe\"}").expect("bare record");
    let bare = plumb::record::read::<Note>(&path, 3);
    assert!(matches!(bare, Err(plumb::record::Error::Absent(_))));
}

#[test]
fn replace() {
    let root = tempfile::tempdir().expect("replace seat");
    let path = root.path().join("held.txt");
    std::fs::write(&path, "old").expect("prior file");
    plumb::replace::write(&path, b"new").expect("replaced");
    assert_eq!(
        std::fs::read_to_string(&path).expect("replaced text"),
        "new"
    );
    plumb::replace::private(&path, b"kept").expect("private replace");
    assert_eq!(
        std::fs::read_to_string(&path).expect("private text"),
        "kept"
    );
    let names = std::fs::read_dir(root.path())
        .expect("seat listing")
        .map(|entry| entry.expect("seat entry").file_name())
        .collect::<Vec<_>>();
    assert_eq!(names, vec![std::ffi::OsString::from("held.txt")]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path)
            .expect("private mode")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
