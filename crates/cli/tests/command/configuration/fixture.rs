use plumb::depot::{sha, v3};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) struct Mock {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Mock {
    pub(super) fn open() -> (TcpListener, String) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let source = format!("http://{}", listener.local_addr().expect("addr"));
        (listener, source)
    }

    pub(super) fn start(listener: TcpListener, routes: BTreeMap<String, Vec<u8>>) -> Self {
        let address = listener.local_addr().expect("addr");
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            for incoming in listener.incoming() {
                if flag.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(mut stream) = incoming else {
                    break;
                };
                reply(&mut stream, &routes);
            }
        });
        Self {
            address,
            stop,
            handle: Some(handle),
        }
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn reply(stream: &mut TcpStream, routes: &BTreeMap<String, Vec<u8>>) {
    let mut buffer = [0u8; 4096];
    let Ok(size) = stream.read(&mut buffer) else {
        return;
    };
    let path = String::from_utf8_lossy(&buffer[..size])
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();
    let (status, body) = match routes.get(&path) {
        Some(held) => ("200 OK", held.as_slice()),
        None => ("404 Not Found", &[][..]),
    };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

fn collect(root: &Path, dir: &Path, bodies: &mut BTreeMap<String, Vec<u8>>) {
    let mut entries = std::fs::read_dir(dir)
        .expect("rules dir")
        .map(|entry| entry.expect("entry").path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect(root, &path, bodies);
        } else {
            let rel = path
                .strip_prefix(root)
                .expect("strip")
                .to_string_lossy()
                .replace('\\', "/");
            bodies.insert(format!("rules/{rel}"), std::fs::read(&path).expect("rule"));
        }
    }
}

pub(super) fn bundle(version: &str, note: Option<&[u8]>) -> v3::Bundle {
    let root = if Path::new("rules").is_dir() {
        Path::new("rules")
    } else {
        Path::new("crates/cli/rules")
    };
    let mut bodies = BTreeMap::new();
    collect(root, root, &mut bodies);
    if let Some(bytes) = note {
        bodies.insert("rules/probe.txt".into(), bytes.to_vec());
    }
    let objects = bodies
        .iter()
        .map(|(path, body)| v3::Object {
            path: path.clone(),
            sha256: sha(body),
            size: body.len() as u64,
            media: if path.ends_with(".toml") {
                "application/toml".into()
            } else {
                "text/plain".into()
            },
            executable: false,
        })
        .collect();
    let identity = v3::Identity {
        product: "plumb".into(),
        channel: "stable".into(),
        version: version.into(),
        marker: v3::Marker {
            name: version.into(),
            sha256: "a".repeat(64),
        },
        kind: v3::Kind::Configuration,
    };
    let manifest = v3::Manifest::new(identity, objects).expect("manifest");
    v3::Bundle { manifest, bodies }
}

pub(super) fn publish(
    routes: &mut BTreeMap<String, Vec<u8>>,
    source: &str,
    held: &v3::Bundle,
) -> v3::Pointer {
    let pointer = v3::Pointer::new(
        &held.manifest,
        v3::Publication {
            source,
            prior: None,
            created: "2026-10-28T00:00:00Z".into(),
        },
    )
    .expect("pointer");
    let route = v3::Route::new("stable", v3::Kind::Configuration, &held.manifest.version);
    let prefix = v3::generation(route, &pointer.generation).expect("prefix");
    routes.insert(
        format!("/{}", v3::latest(route).expect("latest")),
        pointer.encode().expect("pointer"),
    );
    routes.insert(
        format!("/{prefix}/{}", v3::LEAF),
        held.manifest.encode().expect("manifest"),
    );
    for (name, body) in &held.bodies {
        routes.insert(format!("/{prefix}/objects/{name}"), body.clone());
    }
    pointer
}

pub(super) fn dead() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve");
    let source = format!("http://{}", listener.local_addr().expect("addr"));
    drop(listener);
    source
}

pub(super) fn invoke(
    home: &Path,
    args: &[&str],
    source: Option<&str>,
    config: Option<&str>,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_plumb"));
    command
        .args(args)
        .env("PLUMB_HOME", home)
        .env_remove("PLUMB_RULES_SOURCE")
        .env_remove("PLUMB_CONFIGURATION")
        .env_remove("PLUMB_DEPOT_MARK");
    if let Some(source) = source {
        command.env("PLUMB_RULES_SOURCE", source);
    }
    if let Some(config) = config {
        command.env("PLUMB_CONFIGURATION", config);
    }
    command.output().expect("plumb should run")
}
