use crate::shape::depot::{Batch, Draft};
use crate::shape::release::Spec;
use plumb::snapshot::Snapshot;
use std::path::{Path, PathBuf};

pub(super) struct Seat {
    temporary: tempfile::TempDir,
    manifest: plumb::guard::Configuration,
}

impl Seat {
    pub fn new(source: &Path, staged: &Path, target: &str) -> Result<Self, String> {
        Git(source).line(target)?;
        let spec = Spec::read(&staged.join("plumb.toml"))?;
        if spec.product != "plumb" {
            return Err("only Plumb may bootstrap release-line guard configuration".into());
        }
        let depot = spec.derivative(plumb::depot::v3::Kind::Configuration)?;
        let binding = crate::command::release::Product::new(&spec)
            .depot()
            .latest("beta", true)?;
        related(target, &binding.release.version)?;
        let snapshot = Snapshot::read(staged).map_err(|error| error.to_string())?;
        let held = crate::shape::depot::inventory(&snapshot)?;
        let plan = Batch::validation(
            held,
            Draft {
                source: depot.source.clone(),
                release: binding.release.clone(),
                timestamp: crate::command::guard::clock::mark()?,
                commit: Git(staged).commit()?,
            },
        )?;
        let validated = crate::command::release::validate_depot(&spec, &binding, &plan)?;
        let manifest = plumb::guard::Configuration::new(
            target.to_string(),
            plumb::guard::Validator {
                version: validated.version,
                release: binding.release.seal.sha256,
                artifact: validated.artifact,
            },
            plan.manifest.objects.clone(),
        )?;
        let temporary = tempfile::Builder::new()
            .prefix("guard-configuration-")
            .tempdir_in(home()?.join("tmp"))
            .map_err(|error| format!("cannot stage guard configuration: {error}"))?;
        manifest.install(temporary.path(), &plan.bodies)?;
        Ok(Self {
            temporary,
            manifest,
        })
    }

    pub fn mark(&self) -> &str {
        self.manifest.digest()
    }

    pub fn path(&self) -> &Path {
        self.temporary.path()
    }
}

pub(crate) fn related(target: &str, validator: &str) -> Result<(), String> {
    let target = semver::Version::parse(target.trim_start_matches('v'))
        .map_err(|error| format!("cannot parse guard target {target}: {error}"))?;
    let validator = semver::Version::parse(validator.trim_start_matches('v'))
        .map_err(|error| format!("cannot parse released validator {validator}: {error}"))?;
    if (validator.major, validator.minor, validator.patch)
        != (target.major, target.minor, target.patch)
        || validator.pre.is_empty()
    {
        return Err(format!(
            "released validator v{validator} does not belong to v{target}"
        ));
    }
    Ok(())
}

pub(super) fn target(root: &Path) -> Result<Option<String>, String> {
    let governance = Git(root).file("plumb.toml")?;
    let governance: toml::Table = governance
        .parse()
        .map_err(|error| format!("cannot parse plumb.toml: {error}"))?;
    let product = governance
        .get("release")
        .and_then(|held| held.get("product"))
        .and_then(toml::Value::as_str);
    if product != Some("plumb") {
        return Ok(None);
    }
    let text = Git(root).file("Cargo.toml")?;
    let doc: toml::Table = text
        .parse()
        .map_err(|error| format!("cannot parse workspace version: {error}"))?;
    let version = doc
        .get("workspace")
        .and_then(|held| held.get("package"))
        .and_then(|held| held.get("version"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| "workspace declares no package version".to_string())?;
    Ok(Some(format!("v{version}")))
}

struct Git<'a>(&'a Path);

impl Git<'_> {
    fn line(&self, version: &str) -> Result<(), String> {
        let branch = self.run(&["symbolic-ref", "--short", "HEAD"], "read release line")?;
        if branch != format!("release/{version}") {
            return Err(format!(
                "configuration mismatch may bootstrap only on release/{version}, got {branch}"
            ));
        }
        Ok(())
    }

    fn commit(&self) -> Result<String, String> {
        self.run(&["rev-parse", "HEAD"], "read staged release commit")
    }

    fn file(&self, path: &str) -> Result<String, String> {
        self.run(&["show", &format!(":{path}")], "read staged configuration")
    }

    fn run(&self, args: &[&str], action: &str) -> Result<String, String> {
        let output = plumb::config::detached("git")
            .arg("-C")
            .arg(self.0)
            .args(args)
            .output()
            .map_err(|error| format!("cannot run git to {action}: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "cannot {action}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

fn home() -> Result<PathBuf, String> {
    let home = plumb::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))
        .ok_or_else(|| "cannot stage guard configuration: no PLUMB_HOME".to_string())?;
    std::fs::create_dir_all(home.join("tmp"))
        .map_err(|error| format!("cannot create guard temporary root: {error}"))?;
    Ok(home)
}
