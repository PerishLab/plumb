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

#[test]
fn scene() {
    use plumb::depot::v3;
    let source = tempfile::tempdir().expect("source");
    std::fs::write(source.path().join("writer.toml"), "command = []\n").expect("scene");
    let bundle = v3::Bundle::read(
        source.path(),
        v3::Identity {
            product: "seatrial".into(),
            channel: "rc".into(),
            version: "v1.2.3-rc.1".into(),
            marker: v3::Marker {
                name: "v1.2.3-rc.1".into(),
                sha256: "a".repeat(64),
            },
            kind: v3::Kind::Scene,
        },
    )
    .expect("scene material does not judge execution semantics");
    let bytes = bundle.manifest.encode().expect("encoded scene");
    let held = v3::Manifest::parse(&bytes).expect("scene manifest");
    assert_eq!(held.kind, v3::Kind::Scene);
    assert_eq!(held.kind.label(), "scene");
    assert_eq!(
        serde_json::to_string(&held.kind).expect("kind"),
        "\"scene\""
    );
    let pointer = v3::Pointer::new(
        &held,
        v3::Publication {
            source: "https://depot.seatrial.example",
            prior: None,
            created: "2026-10-28T00:00:00Z".into(),
        },
    )
    .expect("pointer");
    assert!(
        pointer
            .manifest
            .url
            .contains("/rc/scenes/versions/v1.2.3-rc.1/")
    );
    pointer.bind(&held, &bytes).expect("binding");
    held.verify("writer.toml", b"command = []\n", false)
        .expect("object");
    assert!(held.verify("writer.toml", b"changed", false).is_err());
    assert!(held.verify("writer.toml", b"command = []\n", true).is_err());
    let mut foreign = held.clone();
    foreign.kind = v3::Kind::Skill;
    assert!(
        pointer
            .bind(&foreign, &foreign.encode().expect("foreign"))
            .is_err()
    );
    let target = tempfile::tempdir().expect("target");
    let placed = v3::install(target.path(), &pointer, &bundle).expect("materialize");
    assert_eq!(
        std::fs::read(placed.join("writer.toml")).expect("scene bytes"),
        b"command = []\n"
    );
}
