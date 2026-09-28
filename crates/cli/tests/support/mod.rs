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
    outside(env!("CARGO_BIN_EXE_plumb"))
}

#[allow(dead_code)]
pub fn outside(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    for (name, _) in std::env::vars() {
        let ambient = name.starts_with("CARGO_") || name.starts_with("RUST");
        if ambient && !["CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN"].contains(&name.as_str()) {
            command.env_remove(name);
        }
    }
    #[cfg(windows)]
    msvc(&mut command);
    command
}

#[cfg(windows)]
fn msvc(command: &mut std::process::Command) {
    let Some(path) = std::env::var_os("PATH") else {
        return;
    };
    let mut vc = None;
    let mut sdk = None;
    for entry in std::env::split_paths(&path) {
        let held = entry.to_string_lossy().to_ascii_lowercase();
        if vc.is_none() && held.contains("\\vc\\tools\\msvc\\") && entry.join("cl.exe").is_file() {
            vc = entry.ancestors().nth(3).map(Path::to_path_buf);
        }
        if sdk.is_none() && held.contains("\\windows kits\\") && entry.join("rc.exe").is_file() {
            let root = entry.ancestors().nth(3).map(Path::to_path_buf);
            let version = entry
                .parent()
                .and_then(Path::file_name)
                .map(|name| name.to_os_string());
            sdk = root.zip(version);
        }
    }
    if let (Some(vc), Some((sdk, version))) = (vc, sdk) {
        let include = [
            vc.join("include"),
            sdk.join("include").join(&version).join("ucrt"),
            sdk.join("include").join(&version).join("um"),
            sdk.join("include").join(&version).join("shared"),
        ];
        let lib = [
            vc.join("lib/x64"),
            sdk.join("lib").join(&version).join("ucrt/x64"),
            sdk.join("lib").join(&version).join("um/x64"),
        ];
        command
            .env("VCToolsInstallDir", &vc)
            .env("WindowsSdkDir", &sdk)
            .env(
                "INCLUDE",
                std::env::join_paths(include).expect("MSVC include path"),
            )
            .env(
                "LIB",
                std::env::join_paths(&lib).expect("MSVC library path"),
            )
            .env(
                "LIBPATH",
                std::env::join_paths(lib).expect("MSVC reference path"),
            );
    }
}

#[allow(dead_code)]
pub fn policy(path: &str) -> String {
    let name = path.strip_prefix("rules/").unwrap_or(path);
    rules(&[name])[0].1.clone()
}
