use super::*;
use std::io::{BufRead, BufReader, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

#[test]
fn shapes() {
    assert_eq!(patterns(b" \n").unwrap(), Vec::<String>::new());
    assert_eq!(patterns(b"null").unwrap(), Vec::<String>::new());
    assert_eq!(patterns(br#""safe-other""#).unwrap(), ["safe-other"]);
    assert_eq!(
        patterns(br#"["safe-other","named@0.2.0"]"#).unwrap(),
        ["safe-other", "named@0.2.0"]
    );
    for bad in [b"not-json".as_slice(), b"[42]", b"{}", b"true"] {
        assert!(patterns(bad).is_err());
    }
}

#[test]
fn provider() {
    if let Some(root) = std::env::var_os("PLUMB_AGE_FIXTURE") {
        exercise(Path::new(&root));
        return;
    }
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("child")).unwrap();
    std::fs::write(
        root.path().join("child/package.json"),
        r#"{"name":"child-fixture","private":true,"dependencies":{"@perishlab/workprobe":"0"}}"#,
    )
    .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let date = ["node", "-p", "new Date().toISOString()"].map(str::to_string);
    let fresh = String::from_utf8(process::run(root.path(), "pnpm", &date).unwrap()).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let server = std::thread::spawn(move || serve(listener, stopped, fresh, false));
    std::fs::write(
        root.path().join(".npmrc"),
        format!("registry={endpoint}/\n@perishlab:registry={endpoint}/\n"),
    )
    .unwrap();
    std::fs::write(root.path().join("package.json"), r#"{"name":"age-fixture","private":true,"scripts":{"check":"node -e \"console.log(12345)\""},"dependencies":{"@perishlab/ageprobe":"0","aged-third":"0","exempt-third":"0","locked-third":"0"}}"#).unwrap();
    std::fs::write(
        root.path().join("pnpm-workspace.yaml"),
        "packages:\n  - child\nminimumReleaseAgeExclude:\n  - exempt-third\n  - named@0.2.0\n",
    )
    .unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "packages::age::tests::provider", "--nocapture"])
        .env("PLUMB_AGE_FIXTURE", root.path())
        .env(
            "NPM_CONFIG_USERCONFIG",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env_remove("WHARF_PACKAGES_TOKEN");
    let result = process::capture(command, "age-fixture");
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
    assert!(result.is_ok(), "{result:?}");
    println!("{}", String::from_utf8(result.unwrap()).unwrap());
}

fn exercise(root: &Path) {
    let policy = std::fs::read(root.join("pnpm-workspace.yaml")).unwrap();
    assert_eq!(
        super::super::registry::latest(root, "npm", "@perishlab/ageprobe").unwrap(),
        "0.2.0"
    );
    let mut argv = [
        "pnpm",
        "install",
        "--lockfile-only",
        "--ignore-scripts",
        "--ignore-pnpmfile",
    ]
    .map(str::to_string)
    .to_vec();
    process::run(root, "packages", &argv).unwrap();
    assert_eq!(resolved(root, "@perishlab/ageprobe"), "0.1.0");
    assert_eq!(importer(root, "child", "@perishlab/workprobe"), "0.1.0");
    assert_eq!(resolved(root, "aged-third"), "0.1.0");
    assert_eq!(resolved(root, "exempt-third"), "0.2.0");
    let locked = resolved(root, "locked-third");
    let wanted =
        ["@perishlab/ageprobe", "@perishlab/workprobe"].map(|name| super::super::Package {
            ecosystem: "npm".into(),
            name: name.into(),
            version: "0.2.0".into(),
        });
    super::super::npm::Lock(root)
        .update(&wanted, false)
        .unwrap();
    super::super::npm::Lock(root).verify(&wanted).unwrap();
    assert_eq!(resolved(root, "@perishlab/ageprobe"), "0.2.0");
    assert_eq!(importer(root, "child", "@perishlab/workprobe"), "0.2.0");
    assert_eq!(resolved(root, "locked-third"), locked);
    assert_eq!(
        std::fs::read(root.join("pnpm-workspace.yaml")).unwrap(),
        policy
    );
    let flags = exclusions(root).unwrap();
    argv = [
        "pnpm",
        "--config.ignore-pnpmfile=true",
        "config",
        "get",
        "minimumReleaseAgeExclude",
        "--json",
    ]
    .map(str::to_string)
    .to_vec();
    argv.extend(flags.clone());
    let effective = process::run(root, "packages", &argv).unwrap();
    assert_eq!(
        patterns(&effective).unwrap(),
        ["exempt-third", "named@0.2.0", "@perishlab/*"]
    );
    println!(
        "effective release-age exclusions: {}",
        String::from_utf8(effective).unwrap()
    );
    argv = [
        "pnpm",
        "update",
        "--recursive",
        "--include-workspace-root",
        "--lockfile-only",
        "--ignore-scripts",
        "--ignore-pnpmfile",
        "--no-save",
    ]
    .map(str::to_string)
    .to_vec();
    argv.extend(flags.clone());
    argv.extend(["aged-third".into(), "exempt-third".into()]);
    process::run(root, "packages", &argv).unwrap();
    assert_eq!(resolved(root, "aged-third"), "0.1.0");
    assert_eq!(resolved(root, "exempt-third"), "0.2.0");
    assert_eq!(
        std::fs::read(root.join("pnpm-workspace.yaml")).unwrap(),
        policy
    );
    super::frozen::verify(root, &flags);
    std::fs::write(
        root.join(".pnpmfile.cjs"),
        "throw new Error('must not execute');",
    )
    .unwrap();
    assert!(exclusions(root).is_ok());
    std::fs::write(root.join("pnpm-workspace.yaml"), "packages: []\n").unwrap();
    assert_eq!(
        exclusions(root).unwrap(),
        ["--config.minimum-release-age-exclude=@perishlab/*"]
    );
    let explicit = "packages: []\nminimumReleaseAge: 15\n";
    std::fs::write(root.join("pnpm-workspace.yaml"), explicit).unwrap();
    assert_eq!(
        serde_json::from_slice::<u64>(&read(root, "minimumReleaseAge").unwrap()).unwrap(),
        15
    );
    let policy = Policy::read(root).unwrap();
    assert_eq!(policy.minimum, Some(15));
    assert!(
        !policy
            .flags
            .iter()
            .any(|flag| flag.starts_with("--config.minimum-release-age="))
    );
    assert_eq!(
        std::fs::read_to_string(root.join("pnpm-workspace.yaml")).unwrap(),
        explicit
    );
}

fn resolved(root: &Path, name: &str) -> String {
    importer(root, ".", name)
}

fn importer(root: &Path, seat: &str, name: &str) -> String {
    let text = std::fs::read_to_string(root.join("pnpm-lock.yaml")).unwrap();
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).unwrap();
    value["importers"][seat]["dependencies"][name]["version"]
        .as_str()
        .unwrap()
        .into()
}

pub(super) fn serve(
    listener: std::net::TcpListener,
    stop: Arc<AtomicBool>,
    fresh: String,
    modified: bool,
) {
    let started = Instant::now();
    while !stop.load(Ordering::Relaxed) && started.elapsed() < Duration::from_secs(120) {
        match listener.accept() {
            Ok((stream, _)) => reply(stream, fresh.trim(), modified),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(error) => panic!("{error}"),
        }
    }
}

fn reply(stream: std::net::TcpStream, fresh: &str, modified: bool) {
    let endpoint = format!("http://{}", stream.local_addr().unwrap());
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let route = line
        .split_whitespace()
        .nth(1)
        .unwrap()
        .trim_start_matches('/');
    let name = route.replace("%2f", "/").replace("%2F", "/");
    line.clear();
    while reader.read_line(&mut line).unwrap() > 2 {
        assert!(!line.to_ascii_lowercase().starts_with("authorization:"));
        line.clear();
    }
    let mut body = super::super::registry::fixture(&name, &endpoint, fresh);
    if modified && !name.ends_with(".tgz") {
        let mut value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        value["modified"] = fresh.into();
        body = serde_json::to_vec(&value).unwrap();
    }
    let mut stream = reader.into_inner();
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .unwrap();
    stream.write_all(&body).unwrap();
}
