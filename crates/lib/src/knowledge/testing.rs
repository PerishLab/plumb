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
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::thread;

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

    #[test]
    fn stress() {
        let root = tempfile::tempdir().expect("fixture");
        let body: &[u8] = b"#!/bin/sh\nexit 0\n";
        safe(root.path().to_path_buf(), body);
        assert!(
            busy(root.path().join("race"), body),
            "held write should yield ETXTBSY"
        );
    }

    fn safe(root: PathBuf, body: &'static [u8]) {
        let writers: Vec<_> = (0..8)
            .map(|index| {
                let path = root.join(format!("safe-{index}"));
                thread::spawn(move || replay(&path, body))
            })
            .collect();
        for writer in writers {
            writer.join().expect("safe writer");
        }
    }

    fn replay(path: &Path, body: &[u8]) {
        for _ in 0..40 {
            let _ = std::fs::remove_file(path);
            executable(path, body);
            assert!(
                Command::new(path)
                    .output()
                    .expect("helper run")
                    .status
                    .success()
            );
        }
    }

    fn busy(path: PathBuf, body: &[u8]) -> bool {
        use std::io::Write;
        let _ = std::fs::remove_file(&path);
        let mut file = std::fs::File::create(&path).expect("create");
        file.write_all(body).expect("write");
        file.set_permissions(std::fs::Permissions::from_mode(0o755))
            .expect("mode");
        file.sync_all().expect("sync");
        let hit = (0..200).any(|_| {
            Command::new(&path)
                .output()
                .err()
                .and_then(|error| error.raw_os_error())
                == Some(26)
        });
        drop(file);
        hit
    }
}
