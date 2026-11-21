use super::{Configuration, Declaration, Settings};
use serde_json::json;
use std::path::Path;

fn declaration<'a>(root: &'a Path, producer: &'static str) -> Declaration<'a> {
    Declaration {
        producer,
        file: root,
        buffer: root.join("state/locus"),
    }
}

fn settings(endpoint: Option<&str>) -> Settings {
    Settings {
        enabled: true,
        endpoint: endpoint.map(str::to_owned),
    }
}

#[test]
fn fallback() {
    let root = tempfile::tempdir().unwrap();
    let declared = declaration(root.path(), "plumb");
    let writer = root.path().join("key");
    let sender = root.path().join("locus");
    let env = || {
        (
            Some("https://host.test".into()),
            Some(writer.clone()),
            Some(sender.clone()),
        )
    };
    let config = Configuration::new(settings(None), None).fallback(env);
    let spec = config.reporter(&declared).unwrap().unwrap();
    assert_eq!(spec.name(), "https");
    assert_eq!(
        spec.options(),
        &json!({
            "endpoint": "https://host.test",
            "key_file": writer,
            "sender": sender,
            "path": root.path().join("state/locus.https/plumb"),
        })
    );
    let defaults = Configuration::new(settings(None), None).fallback(|| (None, None, None));
    let spec = defaults.reporter(&declared).unwrap().unwrap();
    assert_eq!(spec.name(), "api");
    assert_eq!(spec.options()["endpoint"], super::ENDPOINT);
    assert_eq!(spec.options()["path"], json!(declared.buffer));
}

#[test]
fn isolation() {
    let root = tempfile::tempdir().unwrap();
    let declared = declaration(root.path(), "plumb");
    let local = "http://127.0.0.1:8888";
    let config = Configuration::new(settings(Some(local)), None)
        .fallback(|| panic!("a local endpoint shields the ambient credential"));
    let spec = config.reporter(&declared).unwrap().unwrap();
    assert_eq!(spec.name(), "api");
    assert_eq!(
        spec.options(),
        &json!({"endpoint": local, "path": declared.buffer})
    );
    let secure = "https://receiver.test";
    let writer = root.path().join("rotating-key");
    let sender = root.path().join("bin/locus");
    let config: Configuration = serde_json::from_value(json!({
        "enabled":true, "endpoint":secure, "key_file":writer, "sender":sender, "other":1
    }))
    .unwrap();
    let spec = config
        .fallback(|| panic!("an explicit local pair must stay together"))
        .reporter(&declaration(root.path(), "concord"))
        .unwrap()
        .unwrap();
    assert_eq!(spec.options()["key_file"], json!(writer));
    assert_eq!(spec.options()["sender"], json!(sender));
    assert_eq!(
        spec.options()["path"],
        json!(root.path().join("state/locus.https/concord"))
    );
    assert!(!declared.buffer.exists());
}

#[test]
fn refusals() {
    let root = tempfile::tempdir().unwrap();
    let declared = declaration(root.path(), "plumb");
    let writer = root.path().join("key");
    for (endpoint, credential) in [
        (Some("https://example.test"), None),
        (None, Some(writer.clone())),
        (Some("http://127.0.0.1"), Some(writer.clone())),
        (
            Some("https://example.test"),
            Some("untrusted-relative-value".into()),
        ),
    ] {
        let cfg = Configuration::new(settings(endpoint), credential);
        let cfg = cfg.fallback(|| panic!("incomplete local configuration must stay local"));
        let error = cfg.reporter(&declared).unwrap_err();
        assert!(!error.contains("untrusted-relative-value"));
    }
    let cfg = Configuration::new(settings(Some("https://example.test")), Some(writer.clone()));
    assert!(
        cfg.reporter(&declaration(root.path(), "../foreign"))
            .is_err()
    );
    let mut invalid = declaration(root.path(), "plumb");
    invalid.buffer = "relative".into();
    let cfg = Configuration::new(settings(Some("https://example.test")), Some(writer));
    assert!(cfg.reporter(&invalid).is_err());
    let cfg = Configuration::new(settings(None), None).sender(root.path().join("locus"));
    assert!(
        cfg.fallback(|| panic!("a local sender must not borrow ambient credentials"))
            .reporter(&declared)
            .is_err()
    );
    let cfg = Configuration::new(
        settings(Some("https://example.test")),
        Some(root.path().join("key")),
    )
    .sender("relative".into());
    assert!(cfg.reporter(&declared).is_err());
    assert!(!declared.buffer.exists());
}

#[test]
fn disabled() {
    let root = tempfile::tempdir().unwrap();
    let declared = declaration(root.path(), "plumb");
    let cfg = Configuration::new(Settings::default(), Some("invalid".into()));
    assert!(
        cfg.fallback(|| panic!("disabled observation must not load ambient settings"))
            .reporter(&declared)
            .unwrap()
            .is_none()
    );
    assert!(root.path().read_dir().unwrap().next().is_none());
}

#[cfg(unix)]
#[test]
fn encoding() {
    use std::os::unix::ffi::OsStrExt;
    let root = tempfile::tempdir().unwrap();
    let invalid = root.path().join(std::ffi::OsStr::from_bytes(b"bad\xff"));
    let mut declared = declaration(root.path(), "plumb");
    let config = |path| Configuration::new(settings(Some("https://host.test")), Some(path));
    assert!(config(invalid.clone()).reporter(&declared).is_err());
    declared.buffer = invalid;
    assert!(config(root.path().join("key")).reporter(&declared).is_err());
}
