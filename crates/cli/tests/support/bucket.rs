use plumb::depot::sha;
use std::collections::BTreeMap;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct Bucket {
    address: std::net::SocketAddr,
    objects: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    done: Arc<AtomicBool>,
    requests: usize,
    server: thread::JoinHandle<usize>,
}

#[allow(dead_code)]
impl Bucket {
    pub fn open(requests: usize) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        listener
            .set_nonblocking(true)
            .expect("nonblocking listener");
        let address = listener.local_addr().expect("address");
        let objects = Arc::new(Mutex::new(BTreeMap::new()));
        let done = Arc::new(AtomicBool::new(false));
        let shared = Arc::clone(&objects);
        let finished = Arc::clone(&done);
        let server = thread::spawn(move || serve(&listener, requests, &shared, &finished));
        Self {
            address,
            objects,
            done,
            requests,
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
        self.done.store(true, Ordering::SeqCst);
        let served = self.server.join().expect("server");
        assert_eq!(
            served, self.requests,
            "bucket served {served} of {} requests",
            self.requests
        );
    }
}

fn serve(
    listener: &TcpListener,
    requests: usize,
    objects: &Mutex<BTreeMap<String, Vec<u8>>>,
    done: &AtomicBool,
) -> usize {
    let mut served = 0;
    while served < requests {
        match step(listener, objects, done) {
            Step::Served => served += 1,
            Step::Idle => (),
            Step::Done => break,
        }
    }
    served
}

enum Step {
    Served,
    Idle,
    Done,
}

fn step(
    listener: &TcpListener,
    objects: &Mutex<BTreeMap<String, Vec<u8>>>,
    done: &AtomicBool,
) -> Step {
    match listener.accept() {
        Ok((stream, _)) => {
            stream.set_nonblocking(false).expect("blocking stream");
            bucket(stream, objects);
            Step::Served
        }
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => idle(done),
        Err(error) => panic!("accept: {error}"),
    }
}

fn idle(done: &AtomicBool) -> Step {
    if done.load(Ordering::SeqCst) {
        return Step::Done;
    }
    thread::sleep(std::time::Duration::from_millis(5));
    Step::Idle
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
