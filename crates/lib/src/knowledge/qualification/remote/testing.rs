use super::*;
use crate::windtun::qualification::State;
use std::io::{Cursor, Error, ErrorKind};

struct Broken {
    sent: bool,
    error: Option<Error>,
}

impl Read for Broken {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.sent {
            return Err(self.error.take().unwrap());
        }
        self.sent = true;
        let bytes = b"NEVER_SHOW";
        let count = bytes.len().min(buffer.len());
        buffer[..count].copy_from_slice(&bytes[..count]);
        Ok(count)
    }
}

#[test]
fn interrupted() {
    let url =
        "https://releases.plumb.perish.uk/v1/objects/sha256/NEVER_SHOW/token.tar.gz?key=SECRET";
    let material = Material::new(url);
    for kind in [
        ErrorKind::UnexpectedEof,
        ErrorKind::TimedOut,
        ErrorKind::Other,
    ] {
        let fault = material
            .body(
                Broken {
                    sent: false,
                    error: Some(Error::new(kind, "NEVER_SHOW")),
                },
                64,
            )
            .unwrap_err();
        assert_eq!(fault.state, State::Unknown);
        assert!(fault.reason.contains("content object"));
        assert!(
            fault
                .reason
                .contains(&format!("body read failed ({kind:?}"))
        );
        assert!(fault.reason.contains(&format!("{:x}", Sha256::digest(url))));
        assert!(!fault.reason.contains("NEVER_SHOW"));
        assert!(!fault.reason.contains("SECRET"));
        assert!(!fault.reason.contains("https://"));
        assert!(fault.reason.len() < 240);
    }
}

#[test]
fn wrapped() {
    let material = Material::new("https://NEVER_SHOW/SECRET");
    for (error, reason) in [
        (
            ureq::Error::Timeout(ureq::Timeout::Global),
            "time budget exceeded",
        ),
        (
            ureq::Error::BadUri("NEVER_SHOW".into()),
            "Other; cause unknown",
        ),
    ] {
        let fault = material
            .body(
                Broken {
                    sent: false,
                    error: Some(error.into_io()),
                },
                64,
            )
            .unwrap_err();
        assert_eq!(fault.state, State::Unknown);
        assert!(fault.reason.contains("body read failed"));
        assert!(fault.reason.contains(reason));
        assert!(!fault.reason.contains("NEVER_SHOW"));
        assert!(!fault.reason.contains("SECRET"));
    }
}

#[test]
fn refused() {
    let material = Material::new("https://NEVER_SHOW/sensitive");
    let fault = material.body(Cursor::new(b"SECRET"), 1).unwrap_err();
    assert_eq!(fault.state, State::Refused);
    assert!(fault.reason.contains("output budget exceeded"));
    assert!(!fault.reason.contains("SECRET"));
    assert!(!fault.reason.contains("NEVER_SHOW"));
    assert_eq!(material.body(Cursor::new(b"ok"), 2).unwrap(), b"ok");
    assert!(material.body(Cursor::new(b""), 0).unwrap().is_empty());
}

#[test]
fn request() {
    let material = Material::new("http://user:NEVER_SHOW@invalid/SECRET");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .build()
        .into();
    let fault = material.read(&agent, 64).unwrap_err();
    assert_eq!(fault.state, State::Unknown);
    assert!(fault.reason.contains("request failed (cause unknown)"));
    assert!(!fault.reason.contains("NEVER_SHOW"));
    assert!(!fault.reason.contains("SECRET"));
    assert!(!fault.reason.contains("http://"));
}

#[test]
fn observed() {
    let material = Material::new("https://NEVER_SHOW/SECRET");
    let missing = material.failure(ureq::Error::StatusCode(404));
    assert_eq!(missing.state, State::Unknown);
    assert!(missing.reason.contains("request failed (HTTP 404)"));
    let time = material.failure(ureq::Error::Timeout(ureq::Timeout::Global));
    assert_eq!(time.state, State::Unknown);
    assert!(time.reason.contains("time budget exceeded"));
    let opaque = material.failure(ureq::Error::BadUri("NEVER_SHOW".into()));
    assert!(!opaque.reason.contains("NEVER_SHOW"));
    assert!(!opaque.reason.contains("authenticate"));
}

#[test]
fn optional() {
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    use std::time::Duration;
    for status in [200, 404, 503] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/object", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0; 4096];
            assert!(socket.read(&mut request).unwrap() > 0);
            write!(
                socket,
                "HTTP/1.1 {status} Response\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
            )
            .unwrap();
        });
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(2)))
            .build()
            .into();
        let result = Material::new(&url).optional(&agent, 2);
        server.join().unwrap();
        match status {
            200 => assert_eq!(result.unwrap().unwrap(), b"ok"),
            404 => assert!(result.unwrap().is_none()),
            _ => assert_eq!(result.unwrap_err().state, State::Unknown),
        }
    }
}

#[test]
fn contexts() {
    let cases = [
        ("v1/channels/rc.json", "channel pointer"),
        ("v1/releases/rc/v1.0.0-rc.1/seal.json", "release seal"),
        (
            "v1/releases/rc/v1.0.0-rc.1/distribution.json",
            "distribution",
        ),
        (
            "v1/releases/rc/v1.0.0-rc.1/windtun/latest.json",
            "latest attempt",
        ),
        (
            "v1/releases/rc/v1.0.0-rc.1/windtun/attempts/1/pass.json",
            "trial record",
        ),
        ("channels/stable/scenes/latest.json", "scene pointer"),
        (
            "channels/stable/scenes/versions/v1/latest.json",
            "scene standing",
        ),
        (
            "channels/stable/scenes/versions/v1/generations/abc/manifest.json",
            "scene manifest",
        ),
        (
            "channels/stable/scenes/versions/v1/generations/abc/objects/a/b.py",
            "scene object",
        ),
    ];
    for (route, purpose) in cases {
        let url = format!("https://NEVER_SHOW/{route}?token=SECRET#SECRET");
        let material = Material::new(&url);
        assert_eq!(material.purpose, purpose);
        assert!(!material.reason("failed").contains("SECRET"));
    }
    assert_ne!(
        Material::new("https://NEVER_SHOW/a").identity,
        Material::new("https://NEVER_SHOW/b").identity
    );
}
