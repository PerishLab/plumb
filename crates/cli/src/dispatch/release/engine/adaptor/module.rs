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
        self.npm(
            &["pack", "--pack-destination", &seat.to_string_lossy()],
            npm,
        )?;
        let archive = self.archive(npm, &identity);
        if !archive.is_file() {
            return Err(format!("module attachment left no {}", archive.display()));
        }
        Ok(format!("packed module attachment for {version}"))
    }

    pub fn publish(&self, version: &str, token: &str) -> Result<String, String> {
        let Some(npm) = &self.spec.npm else {
            return Ok(format!("{} has no module attachment", self.spec.product));
        };
        if token.trim().is_empty() {
            return Err("PLUMB_RELEASE_REGISTRY_TOKEN is required".into());
        }
        self.pack(version)?;
        let identity = release(version)?;
        self.authenticated(&["publish", "--registry", &npm.registry], npm, token)?;
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

    fn npm(&self, args: &[&str], npm: &super::super::super::model::Npm) -> Result<(), String> {
        self.run(Command::new("npm").args(args).current_dir(self.seat(npm)))
    }

    fn authenticated(
        &self,
        args: &[&str],
        npm: &super::super::super::model::Npm,
        token: &str,
    ) -> Result<(), String> {
        let seat = npm
            .registry
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(&npm.registry);
        let mut command = Command::new("npm");
        command
            .args(args)
            .current_dir(self.seat(npm))
            .env(format!("npm_config_//{seat}:_authToken"), token);
        self.run(&mut command)
    }

    fn run(&self, command: &mut Command) -> Result<(), String> {
        let status = command
            .status()
            .map_err(|error| format!("cannot run npm: {error}"))?;
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

fn bare(package: &str) -> &str {
    package.rsplit('/').next().unwrap_or(package)
}

fn release(version: &str) -> Result<Version, String> {
    Version::parse(version.trim_start_matches('v'))
        .map_err(|error| format!("release version is not semantic: {error}"))
}
