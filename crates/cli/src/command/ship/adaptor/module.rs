use crate::shape::release::Spec;
use base64::Engine;
use semver::Version;
use sha2::{Digest, Sha512};
use std::path::PathBuf;
use std::process::Command;

pub struct Module<'a> {
    pub(super) spec: &'a Spec,
}

pub fn module(spec: &Spec) -> Module<'_> {
    Module { spec }
}

impl Module<'_> {
    pub fn exact(
        &self,
        package: &str,
        version: &str,
        credential: &str,
        reuse: &str,
    ) -> Result<String, String> {
        super::exact::run(
            self,
            super::exact::Request {
                package,
                version,
                credential,
                reuse,
            },
        )
    }

    pub(in crate::command) fn prepare(&self, version: &str) -> Result<(), String> {
        let Some(npm) = &self.spec.npm else {
            return Ok(());
        };
        let identity = release(version)?;
        for package in &npm.packages {
            self.stamp(package, &identity)?;
        }
        Ok(())
    }

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

    pub(super) fn stamp(&self, package: &str, version: &Version) -> Result<(), String> {
        let path = self.seat(package).join("package.json");
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let body = super::projection::stamp(&text, version)
            .map_err(|error| format!("cannot project {}: {error}", path.display()))?;
        std::fs::write(&path, body)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))
    }

    pub(super) fn pnpm(&self, args: &[&str], package: &str) -> Result<(), String> {
        self.run(
            "pnpm",
            Command::new("pnpm")
                .args(args)
                .current_dir(self.seat(package)),
        )
    }

    pub(super) fn carried(
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
        let output = Command::new("npm")
            .args(["view", spec, "dist.integrity", "--registry", &npm.registry])
            .current_dir(cwd)
            .env(format!("npm_config_//{seat}:_authToken"), token)
            .output()
            .map_err(|error| format!("cannot run npm: {error}"))?;
        if !output.status.success() {
            return Ok(None);
        }
        let held = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Some(held).filter(|held| !held.is_empty()))
    }

    pub(super) fn authenticated(
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
        let mut command = Command::new("npm");
        command
            .args(args)
            .current_dir(cwd)
            .env(format!("npm_config_//{seat}:_authToken"), token);
        self.run("npm", &mut command)
    }

    fn run(&self, program: &str, command: &mut Command) -> Result<(), String> {
        let status = command
            .status()
            .map_err(|error| format!("cannot run {program}: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("module attachment command failed".into())
        }
    }

    pub(super) fn seat(&self, package: &str) -> PathBuf {
        self.spec.root.join("packages").join(bare(package))
    }

    pub(super) fn archive(&self, package: &str, version: &Version) -> PathBuf {
        let held = package.replace('@', "").replace('/', "-");
        self.spec
            .root
            .join("target/module")
            .join(format!("{held}-{version}.tgz"))
    }
}

pub(super) fn channel(version: &Version) -> Option<String> {
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

pub(super) fn release(version: &str) -> Result<Version, String> {
    Version::parse(version.trim_start_matches('v'))
        .map_err(|error| format!("release version is not semantic: {error}"))
}

pub(super) fn integrity(archive: &std::path::Path) -> Result<String, String> {
    let bytes = std::fs::read(archive)
        .map_err(|error| format!("cannot read {}: {error}", archive.display()))?;
    let held = base64::engine::general_purpose::STANDARD.encode(Sha512::digest(&bytes));
    Ok(format!("sha512-{held}"))
}

pub(super) fn publication(npm: &crate::shape::release::Npm, package: &str) -> String {
    format!(
        "{}/{}",
        npm.registry.trim_end_matches('/'),
        package.replace('/', "%2f")
    )
}

pub(super) fn drift(spec: &str, carried: &str, held: &str) -> Result<(), String> {
    if carried == held {
        return Ok(());
    }
    Err(format!(
        "published module drift: {spec} holds {carried} while this projection carries {held}"
    ))
}
