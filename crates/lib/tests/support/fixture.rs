use plumb::depot::{sha, v3};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct Server {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    hits: Arc<AtomicUsize>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    pub fn open() -> (TcpListener, String) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let source = format!("http://{}", listener.local_addr().expect("address"));
        (listener, source)
    }

    pub fn start(listener: TcpListener, routes: BTreeMap<String, Vec<u8>>) -> Self {
        let address = listener.local_addr().expect("address");
        let stop = Arc::new(AtomicBool::new(false));
        let hits = Arc::new(AtomicUsize::new(0));
        let flag = Arc::clone(&stop);
        let count = Arc::clone(&hits);
        let handle = std::thread::spawn(move || {
            for incoming in listener.incoming() {
                if flag.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(mut stream) = incoming else {
                    break;
                };
                count.fetch_add(1, Ordering::Relaxed);
                reply(&mut stream, &routes);
            }
        });
        Self {
            address,
            stop,
            hits,
            handle: Some(handle),
        }
    }

    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::Relaxed)
    }
}

impl Drop for Server {
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
    let (head, body) = match routes.get(&path) {
        Some(held) => (
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                held.len()
            ),
            held.as_slice(),
        ),
        None => (
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
            &[][..],
        ),
    };
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

pub fn bundle(channel: &str, version: &str, files: &[(&str, &[u8])]) -> v3::Bundle {
    let objects = files
        .iter()
        .map(|(path, body)| v3::Object {
            path: (*path).into(),
            sha256: sha(body),
            size: body.len() as u64,
            media: "application/toml".into(),
            executable: false,
        })
        .collect::<Vec<_>>();
    let identity = v3::Identity {
        product: "plumb".into(),
        channel: channel.into(),
        version: version.into(),
        marker: v3::Marker {
            name: version.into(),
            sha256: "a".repeat(64),
        },
        kind: v3::Kind::Configuration,
    };
    let manifest = v3::Manifest::new(identity, objects).expect("manifest");
    let bodies = files
        .iter()
        .map(|(path, body)| ((*path).to_string(), body.to_vec()))
        .collect();
    v3::Bundle { manifest, bodies }
}

pub fn publish(
    routes: &mut BTreeMap<String, Vec<u8>>,
    held: &v3::Bundle,
    publication: v3::Publication<'_>,
) -> v3::Pointer {
    let pointer = v3::Pointer::new(&held.manifest, publication).expect("pointer");
    let route = v3::Route::new(
        &held.manifest.channel,
        held.manifest.kind,
        &held.manifest.version,
    );
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

pub fn ask(source: &str) -> v3::Channel<'_> {
    v3::Channel {
        source,
        product: "plumb",
        channel: "stable",
        kind: v3::Kind::Configuration,
    }
}
