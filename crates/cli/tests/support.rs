use plumb::depot::{FORMAT, Manifest, Metadata, Object, Pointer, Schema, sha};
use std::collections::BTreeMap;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

const MARK: &str = "29990101T000000Z";
const SOURCES: [(&str, &str); 3] = [
    ("assets", "assets"),
    ("cookbook", "cookbook"),
    ("help", "help"),
];

#[allow(dead_code)]
pub fn depot(overrides: &[(&str, &str)]) -> tempfile::TempDir {
    let fixture = tempfile::tempdir().expect("depot fixture");
    stock(&fixture.path().join("configurations"), overrides);
    fixture
}

#[allow(dead_code)]
pub fn home(overrides: &[(&str, &str)]) -> tempfile::TempDir {
    let fixture = tempfile::tempdir().expect("home fixture");
    stock(&fixture.path().join(".plumb/configurations"), overrides);
    fixture
}

#[allow(dead_code)]
pub fn guard(overrides: &[(&str, &str)], target: &str) -> tempfile::TempDir {
    let fixture = depot(overrides);
    let base = fixture.path().join("configurations").join(MARK);
    let mut bodies = BTreeMap::new();
    let mut objects = Vec::new();
    for file in walk(&base) {
        let path = file.strip_prefix(&base).expect("guard object");
        if path == Path::new(plumb::depot::LEAF) {
            continue;
        }
        let path = path.to_string_lossy().replace('\\', "/");
        let bytes = std::fs::read(&file).expect("guard body");
        objects.push(Object {
            path: path.clone(),
            sha256: sha(&bytes),
            size: bytes.len() as u64,
        });
        bodies.insert(path, bytes);
    }
    let manifest = plumb::guard::Configuration::new(
        target.into(),
        plumb::guard::Validator {
            version: "v0.0.1".into(),
            release: "a".repeat(64),
            artifact: "b".repeat(64),
        },
        objects,
    )
    .expect("guard configuration");
    manifest
        .install(fixture.path(), &bodies)
        .expect("guard installation");
    fixture
}

pub fn stock(root: &Path, overrides: &[(&str, &str)]) {
    let base = root.join(MARK);
    let mut bodies = skeleton();
    for (source, seat) in SOURCES {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(source);
        for file in walk(&root) {
            let name = file.strip_prefix(&root).expect("relative object");
            let path = format!("{seat}/{}", name.to_string_lossy().replace('\\', "/"));
            bodies.insert(path, std::fs::read(&file).expect("implementation resource"));
        }
    }
    for (path, text) in overrides {
        bodies.insert((*path).into(), text.as_bytes().to_vec());
    }
    let mut objects = Vec::new();
    for (path, bytes) in bodies {
        let target = base.join(&path);
        std::fs::create_dir_all(target.parent().expect("object parent"))
            .expect("object parent seat");
        std::fs::write(target, &bytes).expect("depot object");
        objects.push(Object {
            path,
            sha256: sha(&bytes),
            size: bytes.len() as u64,
        });
    }
    objects.sort();
    let metadata = Metadata {
        version: MARK.to_string(),
        source: "fixture".to_string(),
        channel: "stable".to_string(),
        commit: String::new(),
    };
    let manifest = Manifest {
        schema: Schema {
            format: FORMAT,
            version: concat!("v", env!("CARGO_PKG_VERSION")).to_string(),
        },
        metadata: metadata.clone(),
        objects,
    };
    std::fs::write(
        base.join("plumb.toml"),
        manifest.encode().expect("manifest"),
    )
    .expect("manifest seat");
    std::fs::write(
        root.join("metadata.json"),
        Pointer::new(&metadata, "plumb").encode().expect("pointer"),
    )
    .expect("pointer seat");
}

const TAXONOMY: &str = "[[owner]]\nid = \"fixture\"\nsummary = \"fixture\"\n\n[[tag]]\nid = \"fixture\"\nsummary = \"fixture\"\n";
const POLICY: &str =
    "[limit]\n\n[[shape]]\nwhen = [\"fixture-absent\"]\n\n[[web]]\nseat = \"fixture-absent\"\n";
const STRUCTURE: &str = "[dir]\n\n[lane]\n";
const WORKFLOW: &str = "[suite]\n";
const DEPS: &str = "blacklist = []\n\n[stable.cargo]\nregistry = \"fixture\"\nindex = \"sparse+http://127.0.0.1:9/\"\n";
const RELEASE: &str = "ceiling = 0\n\n[forge]\nimage = \"fixture\"\n\n[permitted]\n\n[exercised]\n";
const SEAT: &str = "[member]\n";
const VOCABULARY: &str = "schema = 1\ncodec = \"p64-v1\"\nretired = []\n";

fn skeleton() -> BTreeMap<String, Vec<u8>> {
    let ids = mechanisms();
    let mut namespaces: Vec<&str> = ids
        .iter()
        .map(|id| id.split('.').next().expect("namespaced mechanism"))
        .collect();
    namespaces.dedup();
    let mut words = TAXONOMY.to_string();
    for namespace in namespaces {
        words.push_str(&format!(
            "\n[[namespace]]\nid = \"{namespace}\"\nsummary = \"fixture\"\nowner = \"fixture\"\n"
        ));
    }
    let mut law = String::new();
    for id in &ids {
        law.push_str(&format!(
            "[[rule]]\nid = \"{id}\"\nsummary = \"fixture\"\nlaw = \"fixture\"\nevidence = \"fixture\"\nstanding = \"mechanized\"\nowner = \"fixture\"\ntags = [\"fixture\"]\n\n"
        ));
    }
    [
        ("rules/taxonomy.toml", words),
        ("rules/catalog.toml", law),
        ("rules/policy.toml", POLICY.to_string()),
        ("rules/structure.toml", STRUCTURE.to_string()),
        ("rules/workflow.toml", WORKFLOW.to_string()),
        ("rules/deps.toml", DEPS.to_string()),
        ("rules/release.toml", RELEASE.to_string()),
        ("rules/seat.toml", SEAT.to_string()),
        ("rules/vocabulary.toml", VOCABULARY.to_string()),
    ]
    .into_iter()
    .map(|(path, text)| (path.to_string(), text.into_bytes()))
    .collect()
}

fn mechanisms() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/catalog/rules");
    let mut ids = Vec::new();
    for file in walk(&root) {
        let text = std::fs::read_to_string(&file).expect("mechanism source");
        for call in text.split("rule!(").skip(1) {
            let args = call.split(')').next().unwrap_or_default();
            if let Some(id) = args.split('"').nth(1) {
                ids.push(id.to_string());
            }
        }
    }
    ids.sort();
    ids.dedup();
    ids
}

#[allow(dead_code)]
pub struct Bucket {
    address: std::net::SocketAddr,
    objects: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    server: thread::JoinHandle<()>,
}

#[allow(dead_code)]
impl Bucket {
    pub fn open(requests: usize) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("address");
        let objects = Arc::new(Mutex::new(BTreeMap::new()));
        let shared = Arc::clone(&objects);
        let server = thread::spawn(move || {
            for stream in listener.incoming().take(requests) {
                bucket(stream.expect("stream"), &shared);
            }
        });
        Self {
            address,
            objects,
            server,
        }
    }

    pub fn endpoint(&self) -> String {
        format!("http://{}", self.address)
    }

    pub fn read(&self, key: &str) -> Vec<u8> {
        self.objects.lock().expect("objects")[key].clone()
    }

    pub fn has(&self, key: &str) -> bool {
        self.objects.lock().expect("objects").contains_key(key)
    }

    pub fn keys(&self) -> Vec<String> {
        self.objects
            .lock()
            .expect("objects")
            .keys()
            .cloned()
            .collect()
    }

    pub fn seed(&self, key: &str, body: &[u8]) {
        self.objects
            .lock()
            .expect("objects")
            .insert(key.to_string(), body.to_vec());
    }

    pub fn finish(self) {
        self.server.join().expect("server");
    }
}

fn bucket(mut stream: TcpStream, objects: &Mutex<BTreeMap<String, Vec<u8>>>) {
    let mut request = Vec::new();
    let mut block = [0u8; 4096];
    let seat = loop {
        let read = stream.read(&mut block).expect("request");
        assert!(read > 0);
        request.extend_from_slice(&block[..read]);
        if let Some(index) = request.windows(4).position(|held| held == b"\r\n\r\n") {
            break index + 4;
        }
    };
    let headers = String::from_utf8_lossy(&request[..seat]).to_string();
    if header(&headers, "x-amz-date").is_some() {
        assert!(
            headers
                .to_ascii_lowercase()
                .contains("authorization: aws4-hmac-sha256")
        );
    }
    let length = header(&headers, "content-length").map_or(0, |held| held.parse().expect("length"));
    while request.len() < seat + length {
        let read = stream.read(&mut block).expect("body");
        assert!(read > 0);
        request.extend_from_slice(&block[..read]);
    }
    let mut line = headers.lines().next().expect("line").split_whitespace();
    let method = line.next().expect("method");
    let path = line.next().expect("path");
    let key = path.strip_prefix("/workflow/").expect("key");
    if let Some(name) = ["if-match", "if-none-match"]
        .into_iter()
        .find(|name| header(&headers, name).is_some())
    {
        let authorization = header(&headers, "authorization").expect("authorization");
        assert!(authorization.contains(name), "{authorization}");
    }
    let mut held = objects.lock().expect("objects");
    let stale = header(&headers, "if-match").is_some_and(|wanted| {
        held.get(key)
            .map(|body| format!("\"{}\"", sha(body)) != wanted)
            .unwrap_or(true)
    });
    let status = match method {
        "HEAD" if held.contains_key(key) => 200,
        "HEAD" => 404,
        "GET" if held.contains_key(key) => 200,
        "GET" => 404,
        "PUT" if stale => 412,
        "PUT" if header(&headers, "if-none-match") == Some("*") && held.contains_key(key) => 412,
        "PUT" => {
            held.insert(key.to_string(), request[seat..seat + length].to_vec());
            200
        }
        _ => 405,
    };
    let body = if status == 200 && method == "GET" {
        held[key].as_slice()
    } else {
        &[]
    };
    let etag = held
        .get(key)
        .map(|body| format!("\"{}\"", sha(body)))
        .unwrap_or_else(|| "\"missing\"".to_string());
    write!(
        stream,
        "HTTP/1.1 {status} held\r\nContent-Length: {}\r\nETag: {etag}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .expect("headers");
    stream.write_all(body).expect("body");
}

fn header<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    headers.lines().find_map(|line| {
        let (held, value) = line.split_once(':')?;
        held.eq_ignore_ascii_case(name).then(|| value.trim())
    })
}

fn walk(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(root).expect("source root") {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            found.extend(walk(&path));
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}
