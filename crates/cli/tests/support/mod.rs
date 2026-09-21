#[path = "bucket.rs"]
mod bucket;
#[allow(unused_imports)]
pub use bucket::Bucket;
use std::path::Path;

#[allow(dead_code)]
pub fn home() -> tempfile::TempDir {
    tempfile::tempdir().expect("home fixture")
}

#[allow(dead_code)]
pub fn overlay(files: &[(&str, &str)]) -> tempfile::TempDir {
    let home = home();
    for (path, body) in files {
        let target = home.path().join("overlay").join(path);
        std::fs::create_dir_all(target.parent().expect("overlay parent")).expect("overlay seat");
        std::fs::write(target, body).expect("overlay rule");
    }
    home
}

#[allow(dead_code)]
pub fn rules(names: &[&str]) -> Vec<(String, String)> {
    names
        .iter()
        .map(|name| {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("rules")
                .join(name);
            let body = std::fs::read_to_string(&path).expect("repository rule source");
            (format!("rules/{name}"), body)
        })
        .collect()
}

#[allow(dead_code)]
pub fn plumb() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_plumb"));
    for (name, _) in std::env::vars() {
        let ambient = name.starts_with("CARGO_") || name.starts_with("RUST");
        if ambient && !["CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN"].contains(&name.as_str()) {
            command.env_remove(name);
        }
    }
    command
}

#[allow(dead_code)]
pub fn policy(path: &str) -> String {
    let name = path.strip_prefix("rules/").unwrap_or(path);
    rules(&[name])[0].1.clone()
}
