use super::super::super::{Provider, State, provider::Patience, pull::Publication, work};
use super::Fixture;
use plumb::depot::{sha, v3};
use std::collections::BTreeMap;
use std::io::{Read, Write};

const NOTE: &[u8] = b"Match Error::Store.\n";

fn lock(keel: &str, locus: &str) -> String {
    let mut text =
        String::from("version = 4\n\n[[package]]\nname = \"fixture\"\nversion = \"0.1.0\"\n");
    for (name, version) in [("keel", keel), ("keel-macro", keel), ("locus", locus)] {
        text.push_str(&format!("\n[[package]]\nname = \"{name}\"\nversion = \"{version}\"\nsource = \"sparse+https://cargo.perish.uk/\"\nchecksum = \"{}\"\n", "0".repeat(64)));
    }
    text
}

fn depot(source: &str) -> (String, BTreeMap<String, Vec<u8>>) {
    let manifest = v3::Manifest::new(
        v3::Identity {
            product: "keel".into(),
            channel: "stable".into(),
            version: "v0.21.0".into(),
            marker: v3::Marker {
                name: "v0.21.0".into(),
                sha256: "a".repeat(64),
            },
            kind: v3::Kind::Changelog,
        },
        vec![v3::Object {
            path: "en/MIGRATION.md".into(),
            sha256: sha(NOTE),
            size: NOTE.len() as u64,
            media: "text/plain".into(),
            executable: false,
        }],
    )
    .unwrap();
    let product = format!("{source}/keel");
    let pointer = v3::Pointer::new(
        &manifest,
        v3::Publication {
            source: &product,
            prior: None,
            created: "2026-10-23T11:30:00Z".into(),
        },
    )
    .unwrap();
    let mut files = BTreeMap::new();
    let path = pointer
        .manifest
        .url
        .strip_prefix(source)
        .unwrap()
        .to_string();
    let base = path.strip_suffix(v3::LEAF).unwrap().to_string();
    files.insert(path, manifest.encode().unwrap());
    files.insert(format!("{base}objects/en/MIGRATION.md"), NOTE.to_vec());
    files.insert(
        "/keel/channels/stable/changelogs/versions/v0.21.0/latest.json".into(),
        pointer.encode().unwrap(),
    );
    for (route, versions) in [
        ("ke/el/keel", ["0.20.3", "0.21.0"]),
        ("ke/el/keel-macro", ["0.20.3", "0.21.0"]),
        ("lo/cu/locus", ["0.20.2", "0.20.3"]),
    ] {
        let lines = versions.map(|version| {
            format!("{{\"name\":\"x\",\"vers\":\"{version}\",\"deps\":[],\"yanked\":false}}\n")
        });
        files.insert(format!("/index/{route}"), lines.concat().into_bytes());
    }
    (pointer.generation, files)
}

fn serve() -> (String, String) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let source = format!("http://{}", listener.local_addr().unwrap());
    let (generation, files) = depot(&source);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut request = [0u8; 4096];
            let count = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..count]);
            let path = request.split_whitespace().nth(1).unwrap_or_default();
            let (status, body) = files
                .get(path)
                .map_or(("404 Not Found", &[][..]), |body| ("200 OK", body));
            let header = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body);
        }
    });
    (source, generation)
}

fn body(fixture: &Fixture) -> String {
    let path = fixture.root.path().join("pull.json");
    let pull: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    pull["body"].as_str().unwrap().to_string()
}

#[test]
fn delta() {
    let fixture = Fixture::new(serde_json::json!({}));
    let source = fixture.repository("pub fn answer() -> u8 {\n    42\n}\n");
    std::fs::write(source.join("Cargo.lock"), lock("0.20.3", "0.20.2")).unwrap();
    for args in [
        vec!["commit", "-qam", "first-party base"],
        vec!["push", "-q", "origin", "main"],
        vec!["fetch", "origin"],
    ] {
        work::git(&source, &args).unwrap();
    }
    let state = State {
        repository: "Example/probe".into(),
        issue: 17,
        worktree: fixture.root.path().join("worktree"),
        ..Default::default()
    };
    work::Work {
        source: &source,
        state: &state,
    }
    .open()
    .unwrap();
    let root = &state.worktree;
    std::fs::write(root.join("Cargo.lock"), lock("0.21.0", "0.20.3")).unwrap();
    work::git(root, &["commit", "-qam", "Refs #17."]).unwrap();
    let head = work::git(root, &["rev-parse", "HEAD"]).unwrap();
    let (served, generation) = serve();
    let provider = Provider {
        command: &fixture.command,
        repository: "Example/probe",
        patience: Patience {
            deadline: std::time::Duration::from_secs(5),
            pause: std::time::Duration::from_millis(50),
        },
        sources: plumb::packages::Sources {
            depot: format!("{served}/{{product}}"),
            index: format!("{served}/index"),
        },
    };
    let publication = || Publication {
        root,
        head: &head,
        previous: None,
    };
    provider.publish(17, publication()).unwrap();
    let text = body(&fixture);
    let reference = format!(
        "MIGRATION {served}/keel/channels/stable/changelogs/versions/v0.21.0/generations/{generation}/objects/en/MIGRATION.md (generation {generation})"
    );
    for expected in [
        "Refs #17.\n",
        "## Delta\n",
        "- cargo keel 0.20.3 → 0.21.0 incompatible\n",
        "- cargo keel-macro 0.20.3 → 0.21.0 incompatible\n",
        "- cargo locus 0.20.2 → 0.20.3 compatible\n  - v0.20.3 changelog: missing (locus Depot)\n",
        &format!("  - v0.21.0 {reference}\n"),
    ] {
        assert!(text.contains(expected), "{expected}\n---\n{text}");
    }
    assert_eq!(text.matches(&reference).count(), 2, "{text}");
    let patches = || {
        fixture
            .calls()
            .iter()
            .filter(|call| call["args"].as_array().unwrap().contains(&"PATCH".into()))
            .count()
    };
    provider.publish(17, publication()).unwrap();
    assert_eq!(patches(), 0);
    let path = fixture.root.path().join("pull.json");
    let mut pull: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    pull["body"] = "Refs #17.\n\n## Outcome\nstale".into();
    std::fs::write(&path, pull.to_string()).unwrap();
    provider.publish(17, publication()).unwrap();
    assert_eq!(patches(), 1);
    assert_eq!(body(&fixture), text);
}
