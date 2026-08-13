use std::{
    fs::OpenOptions,
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    thread,
};

pub fn serve(case: &str, calls: &Path, hits: usize) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("cloud bind");
    let addr = listener.local_addr().expect("cloud address");
    let case = case.to_string();
    let calls = calls.to_path_buf();
    let handle = thread::spawn(move || {
        for _ in 0..hits {
            let (mut stream, _) = listener.accept().expect("cloud accept");
            let request = request(&mut stream);
            let route = request.split_whitespace().nth(1).unwrap_or("");
            let (status, body) = answer(&case, route);
            let reply = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(reply.as_bytes()).expect("cloud reply");
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&calls)
                .expect("calls");
            writeln!(file, "{request}").expect("record");
        }
    });
    (format!("http://{addr}/client/v4"), handle)
}

fn request(stream: &mut std::net::TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut part = [0u8; 1024];
    while let Ok(count) = stream.read(&mut part) {
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&part[..count]);
        if bytes.windows(4).any(|held| held == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&bytes)
        .lines()
        .next()
        .unwrap_or("")
        .to_string()
}

fn answer(case: &str, route: &str) -> (&'static str, &'static str) {
    if route.ends_with("/workers/domains") {
        return match case {
            "unknown" => ("403 Forbidden", r#"{"success":false,"errors":[]}"#),
            "unbound" => ("200 OK", r#"{"success":true,"result":[]}"#),
            _ => (
                "200 OK",
                r#"{"success":true,"result":[{"hostname":"site.test"}]}"#,
            ),
        };
    }
    if route.ends_with("/tokens/verify") {
        return ("200 OK", r#"{"success":true,"result":{"status":"active"}}"#);
    }
    if route.contains("/workers/services/") {
        return ("200 OK", r#"{"success":true,"result":{"id":"probe"}}"#);
    }
    (
        "500 Internal Server Error",
        r#"{"success":false,"errors":[]}"#,
    )
}
