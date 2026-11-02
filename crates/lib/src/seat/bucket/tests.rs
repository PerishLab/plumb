use super::fetch;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread::{self, JoinHandle},
    time::Duration,
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

fn server(status: &'static str, delay: Duration) -> (String, JoinHandle<()>) {
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
        .unwrap();
    });
    (url, worker)
}

#[test]
fn latency() {
    let (url, worker) = server("200 OK", Duration::from_millis(2500));
    let result = fetch(&url);
    worker.join().unwrap();
    assert_eq!(result.unwrap(), Some(b"public-record".to_vec()));
}

#[test]
fn statuses() {
    let (url, worker) = server("404 Not Found", Duration::ZERO);
    assert_eq!(fetch(&url).unwrap(), None);
    worker.join().unwrap();
    let (url, worker) = server("503 Service Unavailable", Duration::ZERO);
    let failure = fetch(&url).unwrap_err();
    worker.join().unwrap();
    assert!(failure.contains("503"));
}
