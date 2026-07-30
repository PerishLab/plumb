use super::model::Spec;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Seat<'a> {
    spec: &'a Spec,
    manager: &'a Path,
    root: &'a Path,
}

pub fn run(spec: &Path, url: &str, version: &str) -> Result<String, String> {
    let spec = Spec::read(spec)?;
    let root = PathBuf::from(format!(".plumb-smoke-{}", std::process::id()));
    if root.exists() {
        return Err(format!("smoke root already exists: {}", root.display()));
    }
    std::fs::create_dir(&root)
        .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
    let result = cycle(&spec, url, version, &root);
    let clean = std::fs::remove_dir_all(&root)
        .map_err(|error| format!("cannot clean {}: {error}", root.display()));
    result?;
    clean?;
    Ok(format!("smoked {} {}", spec.product, version))
}

fn cycle(spec: &Spec, url: &str, version: &str, root: &Path) -> Result<(), String> {
    let manager = root.join(if cfg!(windows) {
        "manage.ps1"
    } else {
        "manage.sh"
    });
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--output",
        ])
        .arg(&manager)
        .arg(url)
        .output()
        .map_err(|error| format!("cannot run curl: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot fetch manager: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let seat = Seat {
        spec,
        manager: &manager,
        root,
    };
    seat.call("install", false)?;
    probe(spec, version, root)?;
    if stable(version) {
        seat.legacy(version)?;
    }
    seat.call("update", false)?;
    probe(spec, version, root)?;
    seat.call("uninstall", true)?;
    if root.join("install").exists() {
        return Err("manager uninstall left its install root".into());
    }
    Ok(())
}

impl Seat<'_> {
    fn legacy(&self, version: &str) -> Result<(), String> {
        let signature = format!("{}-manager-v1", self.spec.product);
        let marker = format!(".{}-manager", self.spec.product);
        let install = self.root.join("install");
        std::fs::write(install.join(&marker), format!("{signature}\n"))
            .map_err(|error| format!("cannot write legacy root marker: {error}"))?;
        std::fs::write(
            install.join(version).join(&marker),
            format!("{signature}\nversion={version}\n"),
        )
        .map_err(|error| format!("cannot write legacy version marker: {error}"))
    }

    fn call(&self, deed: &str, empty: bool) -> Result<(), String> {
        let mut command = if cfg!(windows) {
            let mut held = Command::new("pwsh");
            held.args(["-NoProfile", "-File"]).arg(self.manager);
            held
        } else {
            let mut held = Command::new("sh");
            held.arg(self.manager);
            held
        };
        command
            .arg(deed)
            .env(
                format!("{}_INSTALL_ROOT", self.spec.environment()),
                self.root.join("install"),
            )
            .env(
                format!("{}_LOCAL_BIN_DIR", self.spec.environment()),
                self.root.join("bin"),
            );
        if empty {
            command.args(["--version", ""]);
        }
        let output = command
            .output()
            .map_err(|error| format!("cannot run generated manager: {error}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "manager {deed} failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }
}

fn stable(version: &str) -> bool {
    let held = version.strip_prefix('v').unwrap_or(version);
    let parts = held.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|found| found.is_ascii_digit()))
}

fn probe(spec: &Spec, version: &str, root: &Path) -> Result<(), String> {
    for binary in &spec.binaries {
        let name = if cfg!(windows) {
            format!("{binary}.exe")
        } else {
            binary.clone()
        };
        let output = Command::new(root.join("bin").join(name))
            .arg("--version")
            .output()
            .map_err(|error| format!("cannot probe {binary}: {error}"))?;
        let actual = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let expected = format!("{binary} {version}");
        if !output.status.success() || actual != expected {
            return Err(format!(
                "{binary} version mismatch: expected {expected}, got {actual}"
            ));
        }
    }
    Ok(())
}
