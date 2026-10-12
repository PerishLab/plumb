use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[allow(dead_code)]
pub fn executable(path: &Path, body: &str) {
    let mut child = Command::new("/bin/sh")
        .args(["-c", "cat > \"$1\" && chmod 755 \"$1\"", "fixture"])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .expect("executable writer");
    child
        .stdin
        .take()
        .expect("writer input")
        .write_all(body.as_bytes())
        .expect("executable body");
    assert!(child.wait().expect("writer exit").success());
}
