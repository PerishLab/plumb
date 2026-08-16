use super::super::super::model::Spec;
use semver::Version;
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
        self.stamp(npm, &identity)?;
        let out = self.spec.root.join("target/module");
        std::fs::create_dir_all(&out)
            .map_err(|error| format!("cannot open {}: {error}", out.display()))?;
        let seat = std::fs::canonicalize(&out)
            .map_err(|error| format!("cannot resolve {}: {error}", out.display()))?;
        self.pnpm(
            &["pack", "--pack-destination", &seat.to_string_lossy()],
            npm,
        )?;
        let archive = self.archive(npm, &identity);
        if !archive.is_file() {
            return Err(format!("module attachment left no {}", archive.display()));
        }
        Ok(format!("packed module attachment for {version}"))
    }

    pub fn publish(&self, version: &str, credential: &str) -> Result<String, String> {
        let Some(npm) = &self.spec.npm else {
            return Ok(format!("{} has no module attachment", self.spec.product));
        };
        let token = crate::dispatch::ship::registry_token(credential)?;
        self.pack(version)?;
        let identity = release(version)?;
        let archive = self.archive(npm, &identity);
        let name = archive
            .file_name()
            .ok_or_else(|| format!("packed module has no name: {}", archive.display()))?
            .to_string_lossy()
            .to_string();
        let seat = tempfile::tempdir()
            .map_err(|error| format!("cannot open a module projection seat: {error}"))?;
        std::fs::copy(&archive, seat.path().join(&name))
            .map_err(|error| format!("cannot stage {}: {error}", archive.display()))?;
        if !self.carried(npm, &identity, token, seat.path())? {
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
        }
        self.authenticated(
            &[
                "view",
                &format!("{}@{identity}", npm.package),
                "dist.shasum",
                "--registry",
                &npm.registry,
            ],
            npm,
            token,
            seat.path(),
        )?;
        Ok(format!("published module attachment for {version}"))
    }

    fn stamp(
        &self,
        npm: &super::super::super::model::Npm,
        version: &Version,
    ) -> Result<(), String> {
        let path = self.seat(npm).join("package.json");
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

    fn pnpm(&self, args: &[&str], npm: &super::super::super::model::Npm) -> Result<(), String> {
        self.run(Command::new("pnpm").args(args).current_dir(self.seat(npm)))
    }

    fn carried(
        &self,
        npm: &super::super::super::model::Npm,
        identity: &Version,
        token: &str,
        cwd: &std::path::Path,
    ) -> Result<bool, String> {
        let seat = npm
            .registry
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(&npm.registry);
        let output = Command::new("pnpm")
            .args([
                "view",
                &format!("{}@{identity}", npm.package),
                "version",
                "--registry",
                &npm.registry,
            ])
            .current_dir(cwd)
            .env(format!("npm_config_//{seat}:_authToken"), token)
            .output()
            .map_err(|error| format!("cannot run pnpm: {error}"))?;
        Ok(output.status.success()
            && String::from_utf8_lossy(&output.stdout).trim() == identity.to_string())
    }

    fn authenticated(
        &self,
        args: &[&str],
        npm: &super::super::super::model::Npm,
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

    fn seat(&self, npm: &super::super::super::model::Npm) -> PathBuf {
        self.spec.root.join("packages").join(bare(&npm.package))
    }

    fn archive(&self, npm: &super::super::super::model::Npm, version: &Version) -> PathBuf {
        let held = npm.package.replace('@', "").replace('/', "-");
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
