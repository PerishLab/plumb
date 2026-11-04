use super::fetch;
use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

fn request(stream: &mut TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut received = Vec::new();
    loop {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        received.push(byte[0]);
        if received.ends_with(b"\r\n\r\n") {
            break;
        }
        assert!(received.len() < 16384);
    }
}

fn server(status: &'static str, delay: Duration) -> (String, JoinHandle<io::Result<()>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/record", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        request(&mut stream);
        thread::sleep(delay);
        let body = "public-record";
        write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    });
    (url, worker)
}

#[test]
fn complete() {
    let (url, worker) = server("200 OK", Duration::ZERO);
    let result = fetch(&url);
    let server = worker.join();
    assert_eq!(result, Ok(Some(b"public-record".to_vec())), "{server:?}");
    server.unwrap().unwrap();
}

#[test]
fn latency() {
    let (url, worker) = server("200 OK", Duration::from_millis(2500));
    let started = Instant::now();
    let result = fetch(&url);
    let elapsed = started.elapsed();
    let server = worker.join();
    if let Ok(body) = result {
        assert_eq!(body, Some(b"public-record".to_vec()));
        server.unwrap().unwrap();
    } else {
        timeout(result, server, elapsed);
    }
}

fn timeout(
    result: Result<Option<Vec<u8>>, String>,
    server: thread::Result<io::Result<()>>,
    elapsed: Duration,
) {
    assert_eq!(
        result,
        Err("cannot fetch public bucket object: timeout: global".into()),
        "fixture server: {server:?}"
    );
    assert!(elapsed >= Duration::from_secs(10));
    if let Err(error) = server.expect("client timed out; fixture server must not panic") {
        assert!(matches!(
            error.kind(),
            io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset
        ));
    }
}

#[test]
fn deadline() {
    let (url, worker) = server("200 OK", Duration::from_secs(11));
    let started = Instant::now();
    let result = fetch(&url);
    let elapsed = started.elapsed();
    timeout(result, worker.join(), elapsed);
}

#[test]
fn statuses() {
    let (url, worker) = server("404 Not Found", Duration::ZERO);
    assert_eq!(fetch(&url).unwrap(), None);
    worker.join().unwrap().unwrap();
    let (url, worker) = server("503 Service Unavailable", Duration::ZERO);
    let failure = fetch(&url).unwrap_err();
    worker.join().unwrap().unwrap();
    assert!(failure.contains("503"));
}
