use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub fn executable(path: &Path, mode: &str) {
    assert!(matches!(mode, "basic" | "recovery"));
    let helper = binary().to_str().unwrap().replace('\'', "'\\''");
    plumb::testing::executable(
        path,
        format!("#!/bin/sh\nexec '{helper}' {mode} \"${{0%/*}}\" \"$@\"\n"),
    );
}

fn binary() -> &'static Path {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY.get_or_init(|| {
        let executable = std::env::current_exe().unwrap();
        let path = executable
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples")
            .join("github");
        let recovery = "run cargo build -p plumb-cli --example github --locked";
        assert!(
            path.is_file(),
            "native gh fixture {} is unavailable; {recovery}",
            path.display()
        );
        path
    })
}
