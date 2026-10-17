mod coherent;
use super::process;
pub(super) use coherent::select as coherent;
use semver::{Version, VersionReq};
use std::path::Path;

pub(super) fn latest(root: &Path, ecosystem: &str, name: &str) -> Result<String, String> {
    if ecosystem == "npm" {
        let argv = [
            "pnpm",
            "--config.ignore-pnpmfile=true",
            "view",
            name,
            "dist-tags.latest",
            "--json",
        ]
        .map(str::to_string);
        let bytes = process::run(root, "packages", &argv)?;
        let version: String = serde_json::from_slice(&bytes)
            .map_err(|error| format!("cannot read npm stable {name}: {error}"))?;
        stable(&version)?;
        return Ok(version);
    }
    cargo(&index(root, name)?)
}

pub(super) fn index(root: &Path, name: &str) -> Result<Vec<u8>, String> {
    let url = format!("https://cargo.perish.uk/{}", route(name));
    process::run(root, "probe", &probe(&url))
}

pub(super) fn probe(url: &str) -> [String; 13] {
    [
        "curl",
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--compressed",
        "--retry",
        "1",
        "--connect-timeout",
        "5",
        "--max-time",
        "15",
        url,
    ]
    .map(str::to_string)
}

fn stable(text: &str) -> Result<Version, String> {
    let version =
        Version::parse(text).map_err(|error| format!("invalid stable version {text}: {error}"))?;
    if !version.pre.is_empty() {
        return Err(format!("registry latest {text} is a prerelease"));
    }
    Ok(version)
}

pub(super) struct Entry {
    pub(super) version: Version,
    pub(super) requires: Vec<(String, VersionReq)>,
}

pub(super) fn entries(bytes: &[u8]) -> Result<Vec<Entry>, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    let mut entries = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let item: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| format!("invalid registry line: {error}"))?;
        let text = item
            .get("vers")
            .and_then(serde_json::Value::as_str)
            .ok_or("registry line has no version")?;
        let version = Version::parse(text).map_err(|error| error.to_string())?;
        if item.get("yanked").and_then(serde_json::Value::as_bool) == Some(true)
            || !version.pre.is_empty()
        {
            continue;
        }
        let mut requires = Vec::new();
        for dep in item
            .get("deps")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let field = |key: &str| dep.get(key).and_then(serde_json::Value::as_str);
            if field("kind") == Some("dev") || field("registry").is_some() {
                continue;
            }
            let (Some(name), Some(req)) = (field("package").or(field("name")), field("req")) else {
                return Err(format!("registry line {text} has an unreadable dependency"));
            };
            let req = VersionReq::parse(req).map_err(|error| error.to_string())?;
            requires.push((name.to_string(), req));
        }
        entries.push(Entry { version, requires });
    }
    entries.sort_by(|left, right| right.version.cmp(&left.version));
    Ok(entries)
}

pub(super) fn cargo(bytes: &[u8]) -> Result<String, String> {
    entries(bytes)?
        .first()
        .map(|entry| entry.version.to_string())
        .ok_or_else(|| "registry exposes no stable version".into())
}

fn route(name: &str) -> String {
    let name = name.to_ascii_lowercase();
    match name.len() {
        1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{name}", &name[..1]),
        _ => format!("{}/{}/{name}", &name[..2], &name[2..4]),
    }
}

#[cfg(test)]
pub(super) fn fixture(route: &str, endpoint: &str, fresh: &str) -> Vec<u8> {
    if let Some((name, file)) = route.rsplit_once('/')
        && let Some(version) = file.strip_suffix(".tgz")
    {
        return archive(name, version);
    }
    use base64::Engine;
    use sha2::Digest;
    let versions = ["0.1.0", "0.2.0"].into_iter().map(|version| {
        let digest = sha2::Sha512::digest(archive(route, version));
        let integrity = format!("sha512-{}", base64::engine::general_purpose::STANDARD.encode(digest));
        (version.to_string(), serde_json::json!({"name":route,"version":version,"dist":{"tarball":format!("{endpoint}/{route}/{version}.tgz"),"integrity":integrity}}))
    }).collect::<serde_json::Map<_,_>>();
    serde_json::to_vec(&serde_json::json!({"name":route,"dist-tags":{"latest":"0.2.0"},"versions":versions,"time":{"0.1.0":"2020-01-01T00:00:00Z","0.2.0":fresh}})).unwrap()
}

#[cfg(test)]
fn archive(name: &str, version: &str) -> Vec<u8> {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);
    let bytes = serde_json::to_vec(&serde_json::json!({"name":name,"version":version})).unwrap();
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, "package/package.json", bytes.as_slice())
        .unwrap();
    tar.into_inner().unwrap().finish().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    fn null() -> &'static str {
        if cfg!(windows) { "NUL" } else { "/dev/null" }
    }

    fn socket(listener: TcpListener) -> std::net::TcpStream {
        let started = Instant::now();
        loop {
            match listener.accept() {
                Ok((socket, _)) => return socket,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        started.elapsed() < Duration::from_secs(15),
                        "npm query did not reach fixture registry"
                    );
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(error) => panic!("{error}"),
            }
        }
    }

    fn serve(listener: TcpListener) {
        let mut socket = socket(listener);
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = [0; 8192];
        let size = socket.read(&mut request).unwrap();
        assert!(
            !String::from_utf8_lossy(&request[..size])
                .to_ascii_lowercase()
                .contains("authorization:")
        );
        let body = r#"{"name":"@perishlab/probe","dist-tags":{"latest":"0.2.0"},"versions":{"0.2.0":{"name":"@perishlab/probe","version":"0.2.0"}}}"#;
        write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
    }

    #[test]
    fn npm() {
        if std::env::var_os("PLUMB_REGISTRY_FIXTURE").is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "packages::registry::tests::npm", "--nocapture"])
                .env("PLUMB_REGISTRY_FIXTURE", "child")
                .env("WHARF_PACKAGES_TOKEN", "SYNTHETIC_ONLY_ENV")
                .env("NPM_CONFIG_USERCONFIG", null())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || serve(listener));
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("package.json"),
            r#"{"name":"fixture","private":true}"#,
        )
        .unwrap();
        std::fs::write(
            root.path().join(".npmrc"),
            format!("@perishlab:registry=http://{endpoint}/\n"),
        )
        .unwrap();
        std::fs::write(
            root.path().join(".pnpmfile.cjs"),
            "throw new Error('repository code must not execute during query');",
        )
        .unwrap();
        let result = latest(root.path(), "npm", "@perishlab/probe");
        server.join().unwrap();
        assert_eq!(result.unwrap(), "0.2.0");
    }
}
