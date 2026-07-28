use plumb::skill::{Ask, Kit};
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
            let body = if head.contains("metadata.json") {
                let version = asked(&head);
                format!(
                    r#"{{"releaseVersion":"{version}","artifacts":{{"skillTarGz":{{"name":"plumb.tar.gz","url":"http://127.0.0.1:{port}/plumb.tar.gz","sha256":"{digest}"}}}}}}"#
                )
                .into_bytes()
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

fn asked(head: &str) -> String {
    let Some(seat) = head.find("/versions/") else {
        return "v1.2.3".to_string();
    };
    let rest = &head[seat + "/versions/".len()..];
    rest.split('/').next().unwrap_or("v1.2.3").to_string()
}

fn rig(root: &Path, url: &str) -> Kit {
    fs::create_dir_all(root.join("home").join(".claude").join("skills")).expect("claude");
    Kit {
        name: "plumb".to_string(),
        home: root.join("home"),
        state: root.join("state").join("skills.json"),
        url: url.to_string(),
    }
}

fn ask() -> Ask {
    Ask {
        channel: "stable".to_string(),
        ..Ask::default()
    }
}

fn root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("plumb-skill-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("root");
    path
}

#[test]
fn lands() {
    let archive = pack();
    let digest = plumb::skill::stamp(&archive);
    let url = serve(archive, digest);
    let seat = root("lands");
    let kit = rig(&seat, &url);

    let done = kit.install(&ask()).expect("install");
    assert_eq!(done.kept.len(), 1, "one seat");
    let held = seat
        .join("home")
        .join(".claude")
        .join("skills")
        .join("plumb");
    assert!(held.join("SKILL.md").is_file(), "brief landed");
    assert!(held.join("metadata.json").is_file(), "marker landed");
    let records = kit.list().expect("list");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].version, "v1.2.3");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let state = seat.join("state").join("skills.json");
        assert_eq!(
            fs::metadata(&state).expect("state").permissions().mode() & 0o777,
            0o600
        );
        fs::set_permissions(&state, fs::Permissions::from_mode(0o666)).expect("loosen state");
    }
    let again = kit.install(&ask()).expect("again");
    assert!(again.kept.is_empty(), "second install needs force");
    assert_eq!(again.left.len(), 1);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let state = seat.join("state").join("skills.json");
        assert_eq!(
            fs::metadata(state).expect("state").permissions().mode() & 0o777,
            0o600
        );
    }

    let forced = kit
        .install(&Ask {
            force: true,
            ..ask()
        })
        .expect("forced");
    assert_eq!(forced.kept.len(), 1, "force replaces a managed seat");

    let _ = fs::remove_dir_all(&seat);
}

#[test]
fn guards() {
    let archive = pack();
    let digest = plumb::skill::stamp(&archive);
    let url = serve(archive, digest);
    let seat = root("guards");
    let kit = rig(&seat, &url);
    kit.install(&ask()).expect("install");

    let held = seat
        .join("home")
        .join(".claude")
        .join("skills")
        .join("plumb");
    fs::write(held.join("metadata.json"), "{}").expect("spoil");
    let after = kit
        .install(&Ask {
            force: true,
            ..ask()
        })
        .expect("after");
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
    let held = kit.install(&ask());
    assert!(held.is_err(), "a wrong digest refuses");

    let named = kit.install(&Ask {
        path: Some(seat.join("elsewhere")),
        ..ask()
    });
    assert!(named.is_err(), "an explicit path must end with the name");

    let _ = fs::remove_dir_all(&seat);
}

#[test]
fn climbs() {
    let archive = pack();
    let digest = plumb::skill::stamp(&archive);
    let url = serve(archive, digest);
    let seat = root("climbs");
    let kit = rig(&seat, &url);

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

    let held = seat
        .join("home")
        .join(".claude")
        .join("skills")
        .join("plumb")
        .join("metadata.json");
    let text = fs::read_to_string(held).expect("marker");
    assert!(text.contains("v1.2.3"), "the marker moves with the seat");

    let _ = fs::remove_dir_all(&seat);
}

#[test]
fn preserves_unprefixed_versions() {
    let archive = pack();
    let digest = plumb::skill::stamp(&archive);
    let url = serve(archive, digest);
    let seat = root("unprefixed");
    let kit = rig(&seat, &url);

    kit.install(&Ask {
        version: Some("1.0.0".to_string()),
        ..ask()
    })
    .expect("install");
    assert_eq!(kit.list().expect("list")[0].version, "1.0.0");

    let _ = fs::remove_dir_all(&seat);
}

#[test]
fn prerelease_channels_require_the_exact_version() {
    let seat = root("prerelease-pin");
    let kit = rig(&seat, "http://127.0.0.1:1");
    let loose = kit.install(&Ask {
        channel: "beta".to_string(),
        ..Ask::default()
    });
    assert!(
        matches!(loose, Err(plumb::skill::Error::Floating(channel)) if channel == "beta"),
        "a prerelease latest pointer is discovery, not an install intent"
    );

    let archive = pack();
    let digest = plumb::skill::stamp(&archive);
    let url = serve(archive, digest);
    let kit = rig(&seat, &url);
    kit.install(&Ask {
        channel: "beta".to_string(),
        version: Some("v1.2.3-beta.1".to_string()),
        ..Ask::default()
    })
    .expect("an exact prerelease installs");
    assert_eq!(kit.list().expect("list")[0].version, "v1.2.3-beta.1");

    let _ = fs::remove_dir_all(&seat);
}

#[test]
fn keeps() {
    let archive = pack();
    let digest = plumb::skill::stamp(&archive);
    let url = serve(archive, digest);
    let seat = root("keeps");
    let kit = rig(&seat, &url);
    kit.install(&ask()).expect("install");

    let held = seat
        .join("home")
        .join(".claude")
        .join("skills")
        .join("plumb");
    fs::write(held.join("metadata.json"), "{}").expect("spoil");
    let done = kit.upgrade(&ask()).expect("upgrade");
    assert!(
        done.kept.is_empty(),
        "upgrade does not force past ownership"
    );
    assert!(done.left[0].note.contains("unmanaged"));

    let _ = fs::remove_dir_all(&seat);
}
