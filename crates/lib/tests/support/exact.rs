use plumb::depot::{Rules, Selection, sha, v3};
use std::io::{Read as _, Write as _};
use std::net::TcpListener;

#[test]
fn remote() {
    let body = b"answer = 42\n";
    let manifest = v3::Manifest::new(
        v3::Identity {
            product: "plumb".into(),
            channel: "stable".into(),
            version: "v1.2.3".into(),
            marker: v3::Marker {
                name: "v1.2.3".into(),
                sha256: "a".repeat(64),
            },
            kind: v3::Kind::Configuration,
        },
        vec![v3::Object {
            path: "rules/probe.toml".into(),
            sha256: sha(body),
            size: body.len() as u64,
            media: "application/toml".into(),
            executable: false,
        }],
    )
    .expect("manifest");
    let generation = manifest.generation().expect("generation");
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let source = format!("http://{}", listener.local_addr().expect("address"));
    let route = v3::Route::new("stable", v3::Kind::Configuration, "v1.2.3");
    let base = format!("/{}", v3::generation(route, &generation).expect("route"));
    let responses = std::collections::BTreeMap::from([
        (
            format!("{base}/{}", v3::LEAF),
            manifest.encode().expect("manifest body"),
        ),
        (format!("{base}/objects/rules/probe.toml"), body.to_vec()),
    ]);
    let server = std::thread::spawn(move || {
        for stream in listener.incoming().take(2) {
            let mut stream = stream.expect("stream");
            let mut request = [0u8; 2048];
            let size = stream.read(&mut request).expect("request");
            let path = String::from_utf8_lossy(&request[..size])
                .split_whitespace()
                .nth(1)
                .expect("path")
                .to_string();
            let body = &responses[&path];
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("head");
            stream.write_all(body).expect("body");
        }
    });
    let root = tempfile::tempdir().expect("configuration root");
    let rules = Rules::exact(
        root.path(),
        Selection {
            source: &source,
            channel: "stable",
            version: "v1.2.3",
            generation: &generation,
        },
    )
    .expect("remote rules");
    assert_eq!(rules.mark(), generation);
    assert_eq!(
        rules.read("rules/probe.toml").expect("remote rule"),
        "answer = 42\n"
    );
    assert!(root.path().read_dir().expect("root").next().is_none());
    server.join().expect("server");
}
