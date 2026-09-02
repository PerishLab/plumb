use plumb::bucket::{Condition, Control, Outcome, Policy};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;

fn request(stream: &mut TcpStream) {
    let mut bytes = Vec::new();
    let mut part = [0u8; 1024];
    loop {
        let count = stream.read(&mut part).expect("request");
        bytes.extend_from_slice(&part[..count]);
        let Some(head) = bytes.windows(4).position(|held| held == b"\r\n\r\n") else {
            continue;
        };
        let length = String::from_utf8_lossy(&bytes[..head])
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then_some(value.trim())
            })
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        if bytes.len() >= head + 4 + length {
            return;
        }
    }
}

fn reply(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("reply");
}

fn control(listener: &TcpListener) -> Control {
    Control::new(
        "access".into(),
        "secret".into(),
        "bucket".into(),
        format!("http://{}", listener.local_addr().expect("address")),
    )
    .expect("control")
}

#[test]
fn write() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let control = control(&listener);
    let server = thread::spawn(move || {
        let (mut first, _) = listener.accept().expect("first");
        request(&mut first);
        first.shutdown(Shutdown::Both).expect("interrupt");
        let (mut second, _) = listener.accept().expect("second");
        request(&mut second);
        reply(&mut second, "412 Precondition Failed", "");
    });
    let outcome = control
        .write(
            "records/probe.json",
            b"answer",
            Policy {
                media: "application/json",
                cache: "immutable",
            },
            Condition::Absent,
        )
        .expect("retry");
    assert!(matches!(outcome, Outcome::Stale));
    server.join().expect("server");
}

#[test]
fn read() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let control = control(&listener);
    let server = thread::spawn(move || {
        let (mut first, _) = listener.accept().expect("first");
        request(&mut first);
        reply(&mut first, "503 Service Unavailable", "");
        let (mut second, _) = listener.accept().expect("second");
        request(&mut second);
        reply(&mut second, "200 OK", "answer");
    });
    let outcome = control.read("records/probe.json").expect("retry");
    let Outcome::Held(object) = outcome else {
        panic!("object was not held")
    };
    assert_eq!(object.body, b"answer");
    server.join().expect("server");
}
