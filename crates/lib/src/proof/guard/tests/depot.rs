use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;

use crate::depot::{sha, v3};

pub(super) fn serve(tamper: bool) -> (String, String) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let source = format!("http://{}", listener.local_addr().expect("addr"));
    let body = b"answer = 42\n";
    let manifest = v3::Manifest::new(
        v3::Identity {
            product: "plumb".into(),
            channel: "stable".into(),
            version: "v0.163.0".into(),
            marker: v3::Marker {
                name: "v0.163.0".into(),
                sha256: "a".repeat(64),
            },
            kind: v3::Kind::Configuration,
        },
        vec![v3::Object {
            path: "rules/catalog.toml".into(),
            sha256: sha(body),
            size: body.len() as u64,
            media: "application/toml".into(),
            executable: false,
        }],
    )
    .expect("manifest");
    let pointer = v3::Pointer::new(
        &manifest,
        v3::Publication {
            source: &source,
            prior: None,
            created: "2026-10-28T00:00:00Z".into(),
        },
    )
    .expect("pointer");
    let generation = pointer.generation.clone();
    let route = v3::Route::new("stable", v3::Kind::Configuration, "v0.163.0");
    let prefix = v3::generation(route, &generation).expect("prefix");
    let mut routes = BTreeMap::new();
    let encoded = pointer.encode().expect("pointer");
    routes.insert(
        format!(
            "/{}",
            v3::channel("stable", v3::Kind::Configuration).expect("channel")
        ),
        encoded.clone(),
    );
    routes.insert(format!("/{}", v3::latest(route).expect("latest")), encoded);
    let raw = if tamper {
        b"{}".to_vec()
    } else {
        manifest.encode().expect("manifest")
    };
    routes.insert(format!("/{prefix}/{}", v3::LEAF), raw);
    routes.insert(
        format!("/{prefix}/objects/rules/catalog.toml"),
        body.to_vec(),
    );
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            reply(stream, &routes);
        }
    });
    (source, generation)
}

pub(super) fn dead() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve");
    let addr = listener.local_addr().expect("addr");
    drop(listener);
    format!("http://{addr}")
}

fn reply(mut stream: std::net::TcpStream, routes: &BTreeMap<String, Vec<u8>>) {
    let mut buffer = [0u8; 2048];
    let read = stream.read(&mut buffer).unwrap_or(0);
    let head = String::from_utf8_lossy(&buffer[..read]);
    let path = head.split_whitespace().nth(1).unwrap_or("/");
    let (status, body) = match routes.get(path) {
        Some(held) => ("200 OK", held.as_slice()),
        None => ("404 Not Found", &[][..]),
    };
    let mut out = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    out.extend_from_slice(body);
    let _ = stream.write_all(&out);
}
