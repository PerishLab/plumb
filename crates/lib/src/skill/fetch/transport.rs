use super::Error;
use crate::runtime::exchange;
use std::time::{Duration, Instant};

const ATTEMPTS: u32 = 3;

enum Fault {
    Transient(Option<u16>, String),
    Final(Option<u16>, String),
}

pub(super) fn draw(url: &str) -> Result<Vec<u8>, Error> {
    let started = Instant::now();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(Some(Duration::from_secs(120)))
        .max_idle_connections(0)
        .build()
        .into();
    let mut last = (None, String::new());
    for attempt in 1..=ATTEMPTS {
        let count = attempt as u8;
        match once(&agent, url) {
            Ok(bytes) => {
                exchange::attempt("skill.fetch", "ok", Some(200), (count, started.elapsed()));
                return Ok(bytes);
            }
            Err(Fault::Final(code, why)) => {
                exchange::attempt("skill.fetch", label(code), code, (count, started.elapsed()));
                return Err(Error::Fetch(url.to_string(), why));
            }
            Err(Fault::Transient(code, why)) => last = (code, why),
        }
        if attempt < ATTEMPTS {
            std::thread::sleep(Duration::from_millis(250 * u64::from(attempt)));
        }
    }
    exchange::attempt(
        "skill.fetch",
        "error",
        last.0,
        (ATTEMPTS as u8, started.elapsed()),
    );
    Err(Error::Fetch(
        url.to_string(),
        format!("transport failed after {ATTEMPTS} attempts: {}", last.1),
    ))
}

fn label(code: Option<u16>) -> &'static str {
    if code == Some(404) {
        "missing"
    } else {
        "error"
    }
}

fn once(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, Fault> {
    let response = agent.get(url).call().map_err(|error| match error {
        ureq::Error::StatusCode(code) if code < 500 => Fault::Final(Some(code), error.to_string()),
        ureq::Error::StatusCode(code) => Fault::Transient(Some(code), error.to_string()),
        ureq::Error::Io(_)
        | ureq::Error::Timeout(_)
        | ureq::Error::ConnectionFailed
        | ureq::Error::BodyStalled
        | ureq::Error::Protocol(_) => Fault::Transient(None, error.to_string()),
        other => Fault::Final(None, other.to_string()),
    })?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut response.into_body().as_reader(), &mut bytes)
        .map_err(|error| Fault::Transient(None, error.to_string()))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    fn serve(script: &'static [&'static str]) -> (String, std::thread::JoinHandle<usize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/stable.json", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            for reply in script {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                while reader.read_line(&mut line).unwrap() > 2 {
                    line.clear();
                }
                let mut stream = stream;
                let bytes: &[u8] = match *reply {
                    "cut" => b"",
                    "short" => b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\n\r\n{\"sche",
                    "missing" => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n",
                    _ => b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\n{\"schema\":1}\n",
                };
                stream.write_all(bytes).unwrap();
            }
            script.len()
        });
        (url, server)
    }

    #[test]
    fn retried() {
        let (url, server) = serve(&["cut", "short", "ok"]);
        assert_eq!(super::draw(&url).unwrap(), b"{\"schema\":1}\n");
        assert_eq!(server.join().unwrap(), 3);
    }

    #[test]
    fn bounded() {
        let (url, server) = serve(&["cut", "short", "cut"]);
        let error = super::draw(&url).unwrap_err().to_string();
        assert!(
            error.contains("transport failed after 3 attempts"),
            "{error}"
        );
        assert_eq!(server.join().unwrap(), 3);
    }

    #[test]
    fn definite() {
        let (url, server) = serve(&["missing"]);
        let error = super::draw(&url).unwrap_err().to_string();
        assert!(!error.contains("attempts"), "{error}");
        assert_eq!(server.join().unwrap(), 1);
    }
}
