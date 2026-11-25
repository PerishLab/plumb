use super::{public, wire};
#[cfg(unix)]
use std::process::Command;

#[test]
fn schema() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("ssh_host_ed25519_key.pub");
    std::fs::write(&path, public(&wire(&[7; 32]))).unwrap();
    let instance = plumb::instance::read(&path).unwrap();
    let value = serde_json::to_value(&instance).unwrap();
    assert_eq!(value["schema"], "plumb.instance/v1");
    assert_eq!(value["id"], instance.id);
    assert_eq!(value["source"]["kind"], "ssh-host-key");
    assert_eq!(value["source"]["path"], path.to_str().unwrap());
}

#[test]
fn absent() {
    let root = tempfile::tempdir().unwrap();
    assert!(plumb::instance::read(&root.path().join("missing.pub")).is_err());
    assert!(plumb::instance::read(root.path()).is_err());
    let path = root.path().join("oversize.pub");
    std::fs::write(&path, vec![b'x'; 16 * 1024 + 1]).unwrap();
    assert!(plumb::instance::read(&path).is_err());
}

#[test]
fn private() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("ssh_host_ed25519_key");
    assert!(
        plumb::instance::read(&path)
            .unwrap_err()
            .contains("only .pub")
    );
}

#[test]
#[cfg(unix)]
fn linked() {
    let root = tempfile::tempdir().unwrap();
    let public = root.path().join("ssh_host_ed25519_key.pub");
    std::os::unix::fs::symlink(root.path().join("private"), &public).unwrap();
    assert!(plumb::instance::read(&public).is_err());
}

#[test]
#[cfg(unix)]
fn openssh() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("ssh_host_ed25519_key.pub");
    std::fs::write(&path, public(&wire(&[7; 32]))).unwrap();
    let output = Command::new("ssh-keygen")
        .args(["-lf", path.to_str().unwrap(), "-E", "sha256"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let fingerprint = text.split_whitespace().nth(1).unwrap();
    assert_eq!(
        plumb::instance::read(&path).unwrap().id,
        format!("ssh-ed25519:{fingerprint}")
    );
}
