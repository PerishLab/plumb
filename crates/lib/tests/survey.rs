use plumb::skill::{Action, Ask, Kit, Standing};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct Hits {
    metadata: AtomicUsize,
    archive: AtomicUsize,
}

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

fn serve(archive: Vec<u8>, digest: String) -> (String, Arc<Hits>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let hits = Arc::new(Hits::default());
    let seen = Arc::clone(&hits);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream");
            let mut buffer = [0u8; 1024];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let head = String::from_utf8_lossy(&buffer[..read]);
            let metadata = head.contains("metadata.json");
            if metadata {
                seen.metadata.fetch_add(1, Ordering::SeqCst);
            } else {
                seen.archive.fetch_add(1, Ordering::SeqCst);
            }
            let body = if metadata {
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
    (format!("http://127.0.0.1:{port}"), hits)
}

fn asked(head: &str) -> String {
    let Some(seat) = head.find("/versions/") else {
        return "v1.2.3".to_string();
    };
    let rest = &head[seat + "/versions/".len()..];
    format!("v{}", rest.split('/').next().unwrap_or("1.2.3"))
}

fn rig(root: &Path, url: &str) -> Kit {
    fs::create_dir_all(root.join("home/.claude/skills")).expect("claude");
    Kit {
        name: "plumb".to_string(),
        home: root.join("home"),
        state: root.join("state/skills.json"),
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
    let path = std::env::temp_dir().join(format!("plumb-survey-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("root");
    path
}

#[test]
fn current() {
    let archive = pack();
    let (url, hits) = serve(archive.clone(), plumb::skill::stamp(&archive));
    let seat = root("current");
    let kit = rig(&seat, &url);
    kit.install(&ask()).expect("install");

    hits.metadata.store(0, Ordering::SeqCst);
    hits.archive.store(0, Ordering::SeqCst);
    let report = kit.status(&ask()).expect("status");
    assert_eq!(report.seats[0].state, Standing::Current);
    assert_eq!(report.seats[0].action, Action::None);
    assert_eq!(hits.archive.load(Ordering::SeqCst), 0);

    let done = kit.upgrade(&ask()).expect("no-op");
    assert_eq!(done.same.len(), 1);
    assert_eq!(hits.metadata.load(Ordering::SeqCst), 2);
    assert_eq!(hits.archive.load(Ordering::SeqCst), 0);
    let _ = fs::remove_dir_all(seat);
}

#[test]
fn direction() {
    let archive = pack();
    let (url, hits) = serve(archive.clone(), plumb::skill::stamp(&archive));
    let seat = root("direction");
    let kit = rig(&seat, &url);
    kit.install(&Ask {
        version: Some("1.0.0".to_string()),
        ..ask()
    })
    .expect("old");
    assert_eq!(
        kit.status(&ask()).expect("update").seats[0].action,
        Action::Upgrade
    );
    kit.upgrade(&ask()).expect("upgrade");

    let exact = Ask {
        version: Some("1.0.0".to_string()),
        ..ask()
    };
    assert_eq!(
        kit.status(&exact).expect("rollback").seats[0].action,
        Action::Rollback
    );
    kit.upgrade(&exact).expect("rollback");
    kit.upgrade(&Ask {
        version: Some("2.0.0".to_string()),
        ..ask()
    })
    .expect("ahead");

    hits.archive.store(0, Ordering::SeqCst);
    assert_eq!(
        kit.status(&ask()).expect("regression").seats[0].action,
        Action::Refuse
    );
    assert!(kit.upgrade(&ask()).expect("refusal").kept.is_empty());
    assert_eq!(hits.archive.load(Ordering::SeqCst), 0);
    let _ = fs::remove_dir_all(seat);
}

#[test]
fn drift() {
    let archive = pack();
    let (url, _) = serve(archive.clone(), plumb::skill::stamp(&archive));
    let seat = root("shape");
    let kit = rig(&seat, &url);
    assert!(kit.status(&ask()).expect("unmanaged").seats.is_empty());
    kit.install(&ask()).expect("install");
    let held = seat.join("home/.claude/skills/plumb");

    fs::remove_dir_all(&held).expect("remove");
    assert_eq!(
        kit.status(&ask()).expect("missing").seats[0].action,
        Action::Restore
    );
    kit.upgrade(&ask()).expect("restore");
    fs::write(held.join("metadata.json"), "{}").expect("spoil");
    assert_eq!(
        kit.status(&ask()).expect("ownership").seats[0].state,
        Standing::Ownership
    );

    let changed = {
        let mut bytes = archive;
        bytes.push(0);
        bytes
    };
    let (changed_url, hits) = serve(changed.clone(), plumb::skill::stamp(&changed));
    fs::write(
        held.join("metadata.json"),
        r#"{"schema":1,"keeper":"plumb","name":"plumb","version":"v1.2.3"}"#,
    )
    .expect("restore marker");
    let drift = rig(&seat, &changed_url).status(&ask()).expect("drift");
    assert_eq!(drift.seats[0].state, Standing::Drift);
    assert_eq!(drift.seats[0].action, Action::Refuse);
    assert_eq!(hits.archive.load(Ordering::SeqCst), 0);
    let _ = fs::remove_dir_all(seat);
}
