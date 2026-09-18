#![cfg(feature = "skill")]

use plumb::depot::{sha, v3};
use plumb::skill::{Action, Ask, Kit};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn upgrades() {
    let fixture = tempfile::tempdir().expect("fixture");
    let (source, active) = serve();
    let home = fixture.path().join("home");
    std::fs::create_dir_all(home.join(".claude/skills")).expect("agent");
    let kit = Kit {
        name: "probe".into(),
        home,
        state: fixture.path().join("skills.json"),
        url: source.clone(),
    };
    let depot = kit.depot(&source, "probe", "v1.2.3");
    let ask = Ask {
        channel: "stable".into(),
        ..Ask::default()
    };
    let installed = depot.install(&ask).expect("first generation");
    assert_eq!(installed.kept.len(), 1);
    let path = &installed.kept[0].path;
    active.store(1, Ordering::SeqCst);
    assert_eq!(
        depot.status(&ask).expect("new generation").seats[0].action,
        Action::Upgrade
    );
    std::fs::write(path.join("SKILL.md"), "local edits").expect("local edit");
    assert_eq!(
        depot.status(&ask).expect("preserve edits").seats[0].action,
        Action::Refuse
    );
    assert!(depot.upgrade(&ask).expect("refused update").kept.is_empty());
    assert_eq!(
        std::fs::read_to_string(path.join("SKILL.md")).expect("retained edits"),
        "local edits"
    );
    std::fs::write(path.join("SKILL.md"), "first\n").expect("restore fixture");
    std::fs::write(path.join("extra.txt"), "local addition").expect("local addition");
    assert_eq!(
        depot.status(&ask).expect("preserve addition").seats[0].action,
        Action::Refuse
    );
    std::fs::remove_file(path.join("extra.txt")).expect("remove fixture addition");
    assert_eq!(
        depot.upgrade(&ask).expect("generation update").kept.len(),
        1
    );
    assert_eq!(
        std::fs::read_to_string(path.join("SKILL.md")).expect("updated content"),
        "second\n"
    );
    assert_eq!(
        depot.status(&ask).expect("current generation").seats[0].action,
        Action::None
    );
}

#[test]
fn admits_its_own_prerelease_channel() {
    let fixture = tempfile::tempdir().expect("fixture");
    let home = fixture.path().join("home");
    std::fs::create_dir_all(home.join(".claude/skills")).expect("agent");
    let kit = Kit {
        name: "probe".into(),
        home,
        state: fixture.path().join("skills.json"),
        url: "http://127.0.0.1:9".into(),
    };
    let beta = kit.depot("http://127.0.0.1:9", "probe", "v1.2.3-beta.1");
    assert_eq!(beta.channel(), "beta");
    assert_eq!(
        kit.depot("http://127.0.0.1:9", "probe", "v1.2.3").channel(),
        "stable"
    );
    let asked = |channel: &str| Ask {
        channel: channel.into(),
        ..Ask::default()
    };
    assert!(matches!(
        beta.install(&asked("alpha")),
        Err(plumb::skill::Error::Managed(_))
    ));
    assert!(matches!(
        beta.status(&asked("beta")),
        Err(plumb::skill::Error::Fetch(..))
    ));
}

fn manifest(body: &[u8]) -> v3::Manifest {
    v3::Manifest::new(
        v3::Identity {
            product: "probe".into(),
            channel: "stable".into(),
            version: "v1.2.3".into(),
            marker: v3::Marker {
                name: "v1.2.3".into(),
                sha256: "a".repeat(64),
            },
            kind: v3::Kind::Skill,
        },
        vec![v3::Object {
            path: "SKILL.md".into(),
            sha256: sha(body),
            size: body.len() as u64,
            media: "text/plain".into(),
            executable: false,
        }],
    )
    .expect("manifest")
}

fn serve() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let source = format!("http://{}", listener.local_addr().expect("address"));
    let active = Arc::new(AtomicUsize::new(0));
    let selected = Arc::clone(&active);
    let (pointers, files) = media(&source);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream");
            let mut request = [0u8; 4096];
            let count = stream.read(&mut request).expect("request");
            let request = String::from_utf8_lossy(&request[..count]);
            let path = request.split_whitespace().nth(1).expect("request path");
            let body = if path.ends_with("latest.json") {
                &pointers[selected.load(Ordering::SeqCst)]
            } else {
                files.get(path).expect("known object")
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("header");
            stream.write_all(body).expect("body");
        }
    });
    (source, active)
}

fn media(source: &str) -> (Vec<Vec<u8>>, BTreeMap<String, Vec<u8>>) {
    let mut pointers = Vec::new();
    let mut files = BTreeMap::new();
    let mut prior = None;
    for body in [b"first\n".as_slice(), b"second\n".as_slice()] {
        let manifest = manifest(body);
        let pointer = v3::Pointer::new(
            &manifest,
            v3::Publication {
                source,
                prior,
                created: "2026-09-07T01:00:00Z".into(),
            },
        )
        .expect("pointer");
        let path = pointer
            .manifest
            .url
            .strip_prefix(source)
            .expect("manifest path");
        files.insert(path.to_string(), manifest.encode().expect("manifest body"));
        let base = path.strip_suffix(v3::LEAF).expect("generation path");
        files.insert(format!("{base}objects/SKILL.md"), body.to_vec());
        prior = Some(pointer.generation.clone());
        pointers.push(pointer.encode().expect("pointer body"));
    }
    (pointers, files)
}
