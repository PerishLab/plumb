use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

pub fn read(root: &Path) -> Result<BTreeSet<String>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "--", "Cargo.toml", ":(glob)**/Cargo.toml"])
        .output()
        .map_err(|error| format!("cannot list Cargo manifests: {error}"))?;
    if !output.status.success() {
        return Err("cannot list Cargo manifests for the binary plan".into());
    }
    let listed = String::from_utf8(output.stdout)
        .map_err(|_| "Git listed a non-UTF-8 Cargo manifest".to_string())?;
    let mut roots = BTreeSet::new();
    for path in [
        ".cargo",
        "Cargo.lock",
        "Cargo.toml",
        "plumb.toml",
        "rust-toolchain",
        "rust-toolchain.toml",
    ] {
        if root.join(path).exists() {
            roots.insert(path.to_string());
        }
    }
    for manifest in listed.split('\0').filter(|path| !path.is_empty()) {
        roots.insert(manifest.to_string());
        let seat = Path::new(manifest).parent().unwrap_or(Path::new(""));
        for name in ["src", "build.rs", "assets", "cookbook", "help"] {
            let path = seat.join(name);
            if root.join(&path).exists() {
                roots.insert(display(&path));
            }
        }
    }
    Ok(roots)
}

fn display(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
