use crate::shape::release::Spec;
use base64::Engine;
use semver::Version;
use sha2::{Digest, Sha512};
use std::path::PathBuf;
use std::process::Command;

pub struct Module<'a> {
    spec: &'a Spec,
}

pub fn module(spec: &Spec) -> Module<'_> {
    Module { spec }
}

impl Module<'_> {
    pub fn pack(&self, version: &str) -> Result<String, String> {
        let Some(npm) = &self.spec.npm else {
            return Ok(format!("{} has no module attachment", self.spec.product));
        };
        let identity = release(version)?;
        let out = self.spec.root.join("target/module");
        std::fs::create_dir_all(&out)
            .map_err(|error| format!("cannot open {}: {error}", out.display()))?;
        let seat = std::fs::canonicalize(&out)
            .map_err(|error| format!("cannot resolve {}: {error}", out.display()))?;
        for package in &npm.packages {
            self.stamp(package, &identity)?;
            self.pnpm(
                &["pack", "--pack-destination", &seat.to_string_lossy()],
                package,
            )?;
            let archive = self.archive(package, &identity);
            if !archive.is_file() {
                return Err(format!("module attachment left no {}", archive.display()));
            }
        }
        Ok(format!("packed module attachment for {version}"))
    }

    pub fn publish(&self, version: &str, credential: &str) -> Result<String, String> {
        let Some(npm) = &self.spec.npm else {
            return Ok(format!("{} has no module attachment", self.spec.product));
        };
        let token = crate::command::ship::attachment::credential(credential)?;
        self.pack(version)?;
        let identity = release(version)?;
        for package in &npm.packages {
            let archive = self.archive(package, &identity);
            let name = archive
                .file_name()
                .ok_or_else(|| format!("packed module has no name: {}", archive.display()))?
                .to_string_lossy()
                .to_string();
            let seat = tempfile::tempdir()
                .map_err(|error| format!("cannot open a module projection seat: {error}"))?;
            std::fs::copy(&archive, seat.path().join(&name))
                .map_err(|error| format!("cannot stage {}: {error}", archive.display()))?;
            let spec = format!("{package}@{identity}");
            let held = integrity(&archive)?;
            if let Some(carried) = self.carried(npm, &spec, token, seat.path())? {
                drift(&spec, &carried, &held)?;
                continue;
            }
            let mut publish = vec![
                "publish",
                name.as_str(),
                "--registry",
                npm.registry.as_str(),
            ];
            let channel = channel(&identity);
            if let Some(channel) = &channel {
                publish.extend(["--tag", channel]);
            }
            self.authenticated(&publish, npm, token, seat.path())?;
            let carried = self
                .carried(npm, &spec, token, seat.path())?
                .ok_or_else(|| format!("{spec} reports no integrity after publishing it"))?;
            drift(&spec, &carried, &held)?;
        }
        Ok(format!("published module attachment for {version}"))
    }

    fn stamp(&self, package: &str, version: &Version) -> Result<(), String> {
        let path = self.seat(package).join("package.json");
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let mut held: serde_json::Value = serde_json::from_str(&text)
            .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
        held["version"] = serde_json::Value::String(version.to_string());
        let mut body = serde_json::to_string_pretty(&held)
            .map_err(|error| format!("cannot render {}: {error}", path.display()))?;
        body.push('\n');
        std::fs::write(&path, body)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))
    }

    fn pnpm(&self, args: &[&str], package: &str) -> Result<(), String> {
        self.run(
            Command::new("pnpm")
                .args(args)
                .current_dir(self.seat(package)),
        )
    }

    fn carried(
        &self,
        npm: &crate::shape::release::Npm,
        spec: &str,
        token: &str,
        cwd: &std::path::Path,
    ) -> Result<Option<String>, String> {
        let seat = npm
            .registry
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(&npm.registry);
        let output = Command::new("pnpm")
            .args(["view", spec, "dist.integrity", "--registry", &npm.registry])
            .current_dir(cwd)
            .env(format!("npm_config_//{seat}:_authToken"), token)
            .output()
            .map_err(|error| format!("cannot run pnpm: {error}"))?;
        if !output.status.success() {
            return Ok(None);
        }
        let held = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Some(held).filter(|held| !held.is_empty()))
    }

    fn authenticated(
        &self,
        args: &[&str],
        npm: &crate::shape::release::Npm,
        token: &str,
        cwd: &std::path::Path,
    ) -> Result<(), String> {
        let seat = npm
            .registry
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(&npm.registry);
        let mut command = Command::new("pnpm");
        command
            .args(args)
            .current_dir(cwd)
            .env(format!("npm_config_//{seat}:_authToken"), token);
        self.run(&mut command)
    }

    fn run(&self, command: &mut Command) -> Result<(), String> {
        let status = command
            .status()
            .map_err(|error| format!("cannot run pnpm: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("module attachment command failed".into())
        }
    }

    fn seat(&self, package: &str) -> PathBuf {
        self.spec.root.join("packages").join(bare(package))
    }

    fn archive(&self, package: &str, version: &Version) -> PathBuf {
        let held = package.replace('@', "").replace('/', "-");
        self.spec
            .root
            .join("target/module")
            .join(format!("{held}-{version}.tgz"))
    }
}

fn channel(version: &Version) -> Option<String> {
    version
        .pre
        .split('.')
        .next()
        .filter(|held| !held.is_empty())
        .map(str::to_string)
}

fn bare(package: &str) -> &str {
    package.rsplit('/').next().unwrap_or(package)
}

fn release(version: &str) -> Result<Version, String> {
    Version::parse(version.trim_start_matches('v'))
        .map_err(|error| format!("release version is not semantic: {error}"))
}

fn integrity(archive: &std::path::Path) -> Result<String, String> {
    let bytes = std::fs::read(archive)
        .map_err(|error| format!("cannot read {}: {error}", archive.display()))?;
    let held = base64::engine::general_purpose::STANDARD.encode(Sha512::digest(&bytes));
    Ok(format!("sha512-{held}"))
}

fn drift(spec: &str, carried: &str, held: &str) -> Result<(), String> {
    if carried == held {
        return Ok(());
    }
    Err(format!(
        "published module drift: {spec} holds {carried} while this projection carries {held}"
    ))
}
