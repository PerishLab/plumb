use plumb::skill::{Ask, Error, Kit};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

fn pack() -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    let body = b"---\nname: plumb\n---\n# plumb\n";
    let mut header = tar::Header::new_gnu();
    header.set_size(body.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "plumb/SKILL.md", &body[..])
        .expect("append");
    let tar = builder.into_inner().expect("tar");
    let mut zip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    zip.write_all(&tar).expect("gz");
    zip.finish().expect("finish")
}

fn serve(archive: Vec<u8>, digest: String) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream");
            let mut buffer = [0u8; 1024];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let head = String::from_utf8_lossy(&buffer[..read]);
            let stable = seal(port, &digest, "stable", "v1.2.3");
            let body = if head.contains("/v1/channels/stable.json") {
                format!(
                    r#"{{"schema":1,"channel":"stable","releaseVersion":"v1.2.3","seal":{{"name":"seal.json","url":"http://127.0.0.1:{port}/v1/releases/stable/v1.2.3/seal.json","sha256":"{}"}}}}"#,
                    plumb::skill::stamp(&stable)
                )
                .into_bytes()
            } else if let Some((channel, version)) = route(&head) {
                seal(port, &digest, channel, version)
            } else {
                archive.clone()
            };
            let mut out = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .into_bytes();
            out.extend_from_slice(&body);
            let _ = stream.write_all(&out);
        }
    });
    format!("http://127.0.0.1:{port}")
}

fn seal(port: u16, digest: &str, channel: &str, version: &str) -> Vec<u8> {
    format!(
        r#"{{"schema":1,"channel":"{channel}","releaseVersion":"{version}","artifacts":{{"skill":{{"name":"plumb-skill.tar.gz","url":"http://127.0.0.1:{port}/plumb-skill.tar.gz","sha256":"{digest}"}}}}}}"#
    )
    .into_bytes()
}

fn route(head: &str) -> Option<(&str, &str)> {
    let rest = head.split("/v1/releases/").nth(1)?;
    let mut parts = rest.split('/');
    Some((parts.next()?, parts.next()?))
}

fn rig(root: &Path, url: &str) -> Kit {
    fs::create_dir_all(root.join("home/.claude/skills")).expect("claude");
    Kit {
        name: "plumb".to_string(),
        home: root.join("home"),
        state: root.join("state").join("skills.json"),
        url: url.to_string(),
    }
}

fn seat(tag: &str, dir: &str, url: &str) -> (PathBuf, Kit) {
    let base = root(tag);
    fs::create_dir_all(base.join("home").join(dir)).expect(tag);
    let kit = Kit {
        name: "plumb".to_string(),
        home: base.join("home"),
        state: base.join("state/skills.json"),
        url: url.to_string(),
    };
    (base, kit)
}

fn online(root: &Path) -> Kit {
    let archive = pack();
    let digest = plumb::skill::stamp(&archive);
    rig(root, &serve(archive, digest))
}

fn held(root: &Path) -> PathBuf {
    root.join("home").join(".claude/skills/plumb")
}

fn ask() -> Ask {
    Ask {
        channel: "stable".to_string(),
        ..Ask::default()
    }
}

fn force() -> Ask {
    Ask {
        force: true,
        ..ask()
    }
}

#[cfg(unix)]
fn mode(seat: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(seat.join("state/skills.json"))
        .expect("state")
        .permissions()
        .mode()
        & 0o777
}

fn root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("plumb-skill-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("root");
    path
}

#[test]
fn lands() {
    let seat = root("lands");
    let kit = online(&seat);
    let done = kit.install(&ask()).expect("install");
    assert_eq!(done.kept.len(), 1, "one seat");
    let held = held(&seat);
    assert!(held.join("SKILL.md").is_file(), "brief landed");
    assert!(held.join("metadata.json").is_file(), "marker landed");
    let records = kit.list().expect("list");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].version, "v1.2.3");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(mode(&seat), 0o600);
        fs::set_permissions(
            seat.join("state/skills.json"),
            fs::Permissions::from_mode(0o666),
        )
        .expect("loosen state");
    }
    let again = kit.install(&ask()).expect("again");
    assert!(again.kept.is_empty(), "second install needs force");
    assert_eq!(again.left.len(), 1);
    #[cfg(unix)]
    assert_eq!(mode(&seat), 0o600);
    let forced = kit.install(&force()).expect("forced");
    assert_eq!(forced.kept.len(), 1, "force replaces a managed seat");
    let _ = fs::remove_dir_all(&seat);
}

#[test]
fn discovers() {
    let archive = pack();
    let url = serve(archive.clone(), plumb::skill::stamp(&archive));
    let (dir, kit) = seat("discovers", ".grok/skills", &url);
    let done = kit.install(&ask()).expect("install");
    assert_eq!(done.kept[0].agent, "grok");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn agy() {
    let archive = pack();
    let url = serve(archive.clone(), plumb::skill::stamp(&archive));
    for (tag, dir) in [
        ("presence", ".gemini/antigravity-cli"),
        ("skills", ".gemini/config/skills"),
    ] {
        let (base, kit) = seat(tag, dir, &url);
        let done = kit.install(&ask()).expect("install");
        let target = base.join("home/.gemini/config/skills/plumb");
        assert_eq!(done.kept.len(), 1);
        assert_eq!(done.kept[0].agent, "agy");
        assert_eq!(done.kept[0].path, target);
        assert!(target.join("SKILL.md").is_file());
        assert!(target.join("metadata.json").is_file());
        let _ = fs::remove_dir_all(&base);
    }
    let (empty, kit) = seat("config", ".gemini/config", &url);
    assert!(matches!(kit.install(&ask()), Err(Error::Bare)));
    let _ = fs::remove_dir_all(&empty);
}

#[test]
fn guards() {
    let seat = root("guards");
    let kit = online(&seat);
    kit.install(&ask()).expect("install");
    let held = held(&seat);
    fs::write(held.join("metadata.json"), "{}").expect("spoil");
    let after = kit.install(&force()).expect("after");
    assert!(after.kept.is_empty(), "an unmarked path is not ours");
    assert!(after.left[0].note.contains("unmanaged"));
    let swept = kit.uninstall().expect("uninstall");
    assert!(swept.kept.is_empty(), "uninstall refuses it too");
    assert!(held.is_dir(), "the path survives");
    let _ = fs::remove_dir_all(&seat);
}

#[test]
fn refuses() {
    let archive = pack();
    let url = serve(archive, "0".repeat(64));
    let seat = root("refuses");
    let kit = rig(&seat, &url);
    assert!(kit.install(&ask()).is_err(), "a wrong digest refuses");
    let named = kit.install(&Ask {
        path: Some(seat.join("elsewhere")),
        ..ask()
    });
    assert!(named.is_err(), "an explicit path must end with the name");
    let _ = fs::remove_dir_all(&seat);
}

#[test]
fn climbs() {
    let seat = root("climbs");
    let kit = online(&seat);
    kit.install(&Ask {
        version: Some("v1.0.0".to_string()),
        ..ask()
    })
    .expect("install");
    assert_eq!(kit.list().expect("list")[0].version, "v1.0.0");
    let done = kit.upgrade(&ask()).expect("upgrade");
    assert_eq!(done.kept.len(), 1, "upgrade replaces a managed seat");
    assert!(done.left.is_empty(), "upgrade never skips what it owns");
    assert_eq!(kit.list().expect("list")[0].version, "v1.2.3");
    let text = fs::read_to_string(held(&seat).join("metadata.json")).expect("marker");
    assert!(text.contains("v1.2.3"), "the marker moves with the seat");
    let _ = fs::remove_dir_all(&seat);
}

#[test]
fn prerelease() {
    let seat = root("prerelease-pin");
    let offline = rig(&seat, "http://127.0.0.1:1");
    let beta = Ask {
        channel: "beta".to_string(),
        ..Ask::default()
    };
    assert!(matches!(offline.install(&beta), Err(Error::Managed(_))));
    let kit = online(&seat);
    let loose = kit.stage(&Ask {
        path: Some(seat.join("floating/skills/plumb")),
        ..beta
    });
    assert!(matches!(loose, Err(Error::Floating(_))));
    let staged = seat.join("candidate/skills/plumb");
    let exact = Ask {
        channel: "beta".to_string(),
        version: Some("v1.2.3-beta.1".to_string()),
        path: Some(staged.clone()),
        ..Ask::default()
    };
    let done = kit.stage(&exact).expect("an exact prerelease stages");
    assert_eq!(done.kept.len(), 1);
    assert!(staged.join("SKILL.md").is_file());
    let marker = fs::read_to_string(staged.join("metadata.json")).expect("stage marker");
    assert!(marker.contains(r#""keeper": "plumb-stage""#));
    assert!(kit.list().expect("list").is_empty(), "stage has no ledger");
    assert!(matches!(kit.stage(&exact), Err(Error::Occupied(_))));
    let stable = Ask {
        version: Some("v1.2.3".to_string()),
        path: Some(seat.join("stable/skills/plumb")),
        ..ask()
    };
    assert!(matches!(kit.stage(&stable), Err(Error::Stage)));
    let _ = fs::remove_dir_all(&seat);
}

#[test]
fn keeps() {
    let seat = root("keeps");
    let kit = online(&seat);
    kit.install(&ask()).expect("install");
    let held = held(&seat);
    fs::write(held.join("metadata.json"), "{}").expect("spoil");
    let done = kit.upgrade(&ask()).expect("upgrade");
    assert!(
        done.kept.is_empty(),
        "upgrade does not force past ownership"
    );
    assert!(done.left[0].note.contains("unmanaged"));
    let _ = fs::remove_dir_all(&seat);
}
