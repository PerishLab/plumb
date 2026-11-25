use super::{Lodging, goods, lodged};
use crate::shape::release::Spec;
use plumb::depot::v3::{self, Bundle, Identity, Kind, Marker, Pointer, Publication, Route};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;

struct Case {
    marker: &'static str,
    channel: &'static str,
    corrupt: bool,
    ok: bool,
}

fn serve(listener: TcpListener, routes: BTreeMap<String, Vec<u8>>, limit: usize) {
    std::thread::spawn(move || {
        for incoming in listener.incoming().take(limit) {
            let Ok(mut stream) = incoming else { break };
            let mut buffer = [0u8; 4096];
            let Ok(size) = stream.read(&mut buffer) else {
                continue;
            };
            let path = String::from_utf8_lossy(&buffer[..size])
                .split_whitespace()
                .nth(1)
                .unwrap_or("/")
                .to_string();
            let (head, body) = match routes.get(&path) {
                Some(held) => (
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        held.len()
                    ),
                    held.as_slice(),
                ),
                None => (
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        .into(),
                    &[][..],
                ),
            };
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(body);
        }
    });
}

fn seed(
    routes: &mut BTreeMap<String, Vec<u8>>,
    source: &str,
    version: &str,
    body: &str,
) -> Pointer {
    let fixture = tempfile::tempdir().expect("seed");
    std::fs::write(fixture.path().join("rules.toml"), body).expect("write");
    let identity = Identity {
        product: "plumb".into(),
        channel: "stable".into(),
        version: version.into(),
        marker: Marker {
            name: version.into(),
            sha256: "a".repeat(64),
        },
        kind: Kind::Configuration,
    };
    let bundle = Bundle::read(fixture.path(), identity).expect("bundle");
    let pointer = Pointer::new(
        &bundle.manifest,
        Publication {
            source,
            prior: None,
            created: "2026-10-28T00:00:00Z".into(),
        },
    )
    .expect("pointer");
    let route = Route::new("stable", Kind::Configuration, version);
    let prefix = v3::generation(route, &pointer.generation).expect("prefix");
    routes.insert(
        format!("/{}", v3::latest(route).expect("latest")),
        pointer.encode().expect("pointer"),
    );
    routes.insert(
        format!("/{prefix}/{}", v3::LEAF),
        bundle.manifest.encode().expect("manifest"),
    );
    for (name, bytes) in bundle.bodies {
        routes.insert(format!("/{prefix}/objects/{name}"), bytes);
    }
    pointer
}

fn spec(root: &Path, source: &str) -> Spec {
    let text = format!(
        "[identity]\nns = \"plumb\"\n[release]\nauthority = \"https://releases.plumb.example\"\nbinaries = [\"plumb\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n[release.depot]\nsource = \"{source}\"\n"
    );
    Spec::decode(root, &text, "fixture").expect("spec")
}

#[test]
fn channel() {
    let fixture = tempfile::tempdir().expect("fixture");
    let body = "answer = 42\n";
    let objects = [goods::good("rules.toml".into(), body.as_bytes(), false)];
    let key = format!(
        "/{}",
        v3::channel("stable", Kind::Configuration).expect("key")
    );
    let cases = [
        Case {
            marker: "v1.2.0",
            channel: "v1.2.0",
            corrupt: false,
            ok: true,
        },
        Case {
            marker: "v1.2.0",
            channel: "v1.3.0",
            corrupt: false,
            ok: true,
        },
        Case {
            marker: "v1.2.0",
            channel: "v1.1.0",
            corrupt: false,
            ok: false,
        },
        Case {
            marker: "v1.2.0",
            channel: "v1.2.0",
            corrupt: true,
            ok: false,
        },
    ];
    for case in cases {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let source = format!("http://{}", listener.local_addr().expect("addr"));
        let mut routes = BTreeMap::new();
        let standing = seed(&mut routes, &source, case.marker, body);
        let active = if case.channel == case.marker {
            standing
        } else {
            seed(&mut routes, &source, case.channel, body)
        };
        routes.insert(
            key.clone(),
            if case.corrupt {
                b"{bad".to_vec()
            } else {
                active.encode().expect("encode")
            },
        );
        serve(listener, routes, if case.corrupt { 3 } else { 5 });
        let declared = spec(fixture.path(), &source);
        let outcome = lodged(
            &Lodging {
                spec: &declared,
                channel: "stable",
                marker: case.marker,
                kind: Kind::Configuration,
            },
            &objects,
        );
        assert_eq!(outcome.is_ok(), case.ok, "{}/{}", case.marker, case.channel);
    }
}

#[test]
fn drift() {
    let fixture = tempfile::tempdir().expect("fixture");
    let body = "answer = 42\n";
    let objects = [goods::good("rules.toml".into(), body.as_bytes(), false)];
    let key = format!(
        "/{}",
        v3::channel("stable", Kind::Configuration).expect("key")
    );
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let source = format!("http://{}", listener.local_addr().expect("addr"));
    let mut routes = BTreeMap::new();
    let older = seed(&mut routes, &source, "v1.2.0", "answer = 1\n");
    let _standing = seed(&mut routes, &source, "v1.2.0", body);
    routes.insert(key, older.encode().expect("older"));
    serve(listener, routes, 5);
    let declared = spec(fixture.path(), &source);
    let error = lodged(
        &Lodging {
            spec: &declared,
            channel: "stable",
            marker: "v1.2.0",
            kind: Kind::Configuration,
        },
        &objects,
    )
    .expect_err("channel generation disagrees with standing");
    assert!(error.contains("disagrees"), "{error}");
}
