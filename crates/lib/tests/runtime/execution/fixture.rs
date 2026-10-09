use plumb::config::Search;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub fn binary() -> &'static Path {
    static ROOT: OnceLock<tempfile::TempDir> = OnceLock::new();
    let root = ROOT.get_or_init(|| {
        let root = tempfile::Builder::new().prefix("space 中文 ").tempdir().unwrap();
        let source = root.path().join("probe.rs");
        std::fs::write(&source, r#"fn main() { let args = std::env::args().skip(1).collect::<Vec<_>>(); if args.first().map(String::as_str) == Some("--exit") { std::process::exit(37); } println!("{args:?}"); }"#).unwrap();
        let output = plumb::config::search(root.path()).unwrap()
            .command("rustc").unwrap()
            .arg(&source).arg("-o").arg(root.path().join("probe.exe"))
            .output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        root
    });
    root.path()
}

pub fn tool(root: &Path, name: &str) -> PathBuf {
    let path = root.join(name);
    std::fs::hard_link(binary().join("probe.exe"), &path).unwrap();
    path
}

pub fn search(root: &Path) -> Search {
    Search::new(
        root,
        Some(root.as_os_str().to_owned()),
        Some(".EXE;.CMD;.BAT;.COM".into()),
    )
    .unwrap()
}

pub fn arguments() -> [&'static str; 9] {
    [
        "space value",
        "中文",
        "amp&value",
        "caret^value",
        "percent%TOKEN%",
        "pipe|value",
        "quote\"value",
        "trail\\",
        "",
    ]
}
