use std::path::Path;

#[cfg(unix)]
pub fn executable(path: &Path, body: impl AsRef<[u8]>) {
    use std::io::Write;
    use std::process::{Command, Stdio};

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
        .write_all(body.as_ref())
        .expect("executable body");
    assert!(
        child.wait().expect("writer exit").success(),
        "executable writer failed for {}",
        path.display()
    );
}

pub fn link(source: &Path, path: &Path) {
    std::fs::hard_link(source, path).unwrap_or_else(|error| {
        panic!(
            "hard_link {} -> {}: {error}",
            source.display(),
            path.display()
        )
    });
}

#[cfg(all(test, unix))]
mod tests {
    use super::{executable, link};
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    #[test]
    fn writes() {
        let root = tempfile::tempdir().expect("fixture");
        let path = root.path().join("probe");
        executable(&path, "#!/bin/sh\necho ok\n");
        let mode = std::fs::metadata(&path).expect("meta").permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "{mode:#o}");
        let output = Command::new(&path).output().expect("run");
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ok");
    }

    #[test]
    fn links() {
        let root = tempfile::tempdir().expect("fixture");
        let source = root.path().join("source");
        let path = root.path().join("linked");
        executable(&source, "#!/bin/sh\necho linked\n");
        link(&source, &path);
        let output = Command::new(&path).output().expect("run");
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "linked");
    }
}
