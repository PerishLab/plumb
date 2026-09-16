use super::Fixture;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Output};

#[path = "failure.rs"]
mod failure;

pub struct Server {
    pub source: String,
}

impl Server {
    pub fn new(root: &Path) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener");
        let source = format!("http://{}", listener.local_addr().expect("address"));
        let root = root.join("depot");
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.expect("stream");
                let mut bytes = [0; 8192];
                let count = stream.read(&mut bytes).expect("request");
                let text = String::from_utf8_lossy(&bytes[..count]);
                let path = text.split_whitespace().nth(1).expect("path");
                let (status, body) = match std::fs::read(root.join(path.trim_start_matches('/'))) {
                    Ok(bytes) => ("200 OK", bytes),
                    Err(_) => ("404 Not Found", vec![]),
                };
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .expect("headers");
                stream.write_all(&body).expect("body");
            }
        });
        Self { source }
    }
}

fn command(fixture: &Fixture<'_>) -> Command {
    let mut command = fixture.command();
    command
        .current_dir(fixture.root)
        .env("PLUMB_DEPOT_AUTHORITY_ACCESS", "access")
        .env("PLUMB_DEPOT_AUTHORITY_SECRET", "secret")
        .env("PLUMB_DEPOT_AUTHORITY_BUCKET", "depot")
        .env("PLUMB_DEPOT_AUTHORITY_ENDPOINT", "https://s3.test")
        .args(["depot", "configuration", ".", "--marker", "v1.2.0-beta.2"]);
    command
}

fn success(output: Output) -> Vec<u8> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

pub fn exercise(fixture: &Fixture<'_>, source: &Path) {
    let base = fixture
        .root
        .join("depot/channels/beta/configurations/versions/v1.2.0-beta.2");
    let path = base.join("latest.json");
    let original = std::fs::read(&path).expect("original latest");
    std::fs::write(source.join("rules/probe.toml"), "answer = 43\n").expect("candidate");
    let receipt = success(
        command(fixture)
            .args(["--stage", "--from"])
            .arg(source)
            .output()
            .unwrap(),
    );
    let receipt: serde_json::Value = serde_json::from_slice(&receipt).expect("receipt");
    let generation = receipt["generation"].as_str().expect("generation");
    let expected = receipt["expected"].as_str().expect("expected");
    assert_eq!(expected, plumb::depot::sha(&original));
    assert_eq!(receipt["activated"], false);
    isolation(fixture, generation);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let repeated = success(
        command(fixture)
            .args(["--stage", "--from"])
            .arg(source)
            .output()
            .unwrap(),
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&repeated).unwrap(),
        receipt
    );
    let failed = command(fixture)
        .args(["--promote", generation, "--expect", "absent"])
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("latest changed"));
    assert_eq!(std::fs::read(&path).unwrap(), original);
    failure::before(fixture, &path, generation, expected);
    let lost = command(fixture)
        .env("FAKE_DEPOT_LOST", "true")
        .args(["--promote", generation, "--expect", expected])
        .output()
        .unwrap();
    assert!(!lost.status.success());
    assert!(String::from_utf8_lossy(&lost.stderr).contains("Connection broken after write"));
    success(
        command(fixture)
            .args(["--promote", generation, "--expect", expected])
            .output()
            .unwrap(),
    );
    let advanced = std::fs::read(&path).unwrap();
    let pointer = plumb::depot::v3::Pointer::parse(&advanced).unwrap();
    assert_eq!(pointer.generation, generation);
    assert_eq!(
        pointer.prior,
        Some(
            plumb::depot::v3::Pointer::parse(&original)
                .unwrap()
                .generation
        )
    );
    success(
        command(fixture)
            .args(["--promote", generation, "--expect", expected])
            .output()
            .unwrap(),
    );
    assert_eq!(std::fs::read(&path).unwrap(), advanced);
    let object = base
        .join("generations")
        .join(generation)
        .join("objects/rules/probe.toml");
    std::fs::write(&object, "corrupted").unwrap();
    let failed = command(fixture)
        .args(["--promote", generation, "--expect", expected])
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert_eq!(std::fs::read(&path).unwrap(), advanced);
    let failed = command(fixture)
        .args(["--stage", "--from"])
        .arg(source)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("immutable depot object drift"));
    assert_eq!(std::fs::read(&path).unwrap(), advanced);
}

fn isolation(fixture: &Fixture<'_>, generation: &str) {
    let manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(fixture.root.join("plumb.toml")).unwrap()).unwrap();
    let source = manifest["release"]["depot"]["source"].as_str().unwrap();
    let path = fixture.root.join("candidate");
    let absent = fixture
        .command()
        .current_dir(fixture.root)
        .args([
            "configuration",
            "install",
            "--marker",
            "v1.2.0-beta.2",
            "--generation",
            generation,
        ])
        .output()
        .unwrap();
    assert!(!absent.status.success());
    assert!(String::from_utf8_lossy(&absent.stderr).contains("--path"));
    let foreign = fixture
        .command()
        .env("PLUMB_RULES_SOURCE", source)
        .current_dir(fixture.root)
        .args([
            "configuration",
            "install",
            "--marker",
            "v1.2.0-beta.2",
            "--generation",
            generation,
            "--path",
        ])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!foreign.status.success());
    assert!(
        String::from_utf8_lossy(&foreign.stderr).contains("does not bind the requested projection")
    );
    assert!(!path.exists());
    std::fs::create_dir(&path).unwrap();
    let existing = fixture
        .command()
        .current_dir(fixture.root)
        .args([
            "configuration",
            "install",
            "--marker",
            "v1.2.0-beta.2",
            "--generation",
            generation,
            "--path",
        ])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!existing.status.success());
    assert!(String::from_utf8_lossy(&existing.stderr).contains("requires a new isolated --path"));
    assert!(std::fs::read_dir(path).unwrap().next().is_none());
}
