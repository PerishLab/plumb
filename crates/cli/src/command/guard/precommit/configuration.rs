use crate::shape::depot::{Batch, Draft};
use crate::shape::release::Spec;
use plumb::snapshot::Snapshot;
use std::path::{Path, PathBuf};

pub(super) struct Seat {
    temporary: tempfile::TempDir,
    manifest: plumb::guard::Configuration,
    transaction: bool,
}

impl Seat {
    pub fn new(source: &Path, staged: &Path, target: &str) -> Result<Self, String> {
        let context = Git(source).context(target)?;
        let transaction = context == Context::Line;
        let spec = Spec::read(&staged.join("plumb.toml"))?;
        if spec.product != "plumb" {
            return Err("only Plumb may bootstrap release-line guard configuration".into());
        }
        let depot = spec.derivative(plumb::depot::v3::Kind::Configuration)?;
        let product = crate::command::release::Product::new(&spec);
        let source = product.depot();
        let binding = match context {
            Context::Line => source.validator(target, true)?,
            Context::Source => source.latest("stable", true)?,
        };
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
        let recovery = plumb::config::value("PLUMB_GUARD_RECOVERY_VALIDATOR").map(PathBuf::from);
        let validated =
            crate::command::release::validate_depot(&spec, &binding, &plan, recovery.as_deref())?;
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
            transaction,
        })
    }

    pub fn mark(&self) -> &str {
        self.manifest.digest()
    }

    pub fn product(&self, root: &Path) -> Result<crate::shape::product::Target, String> {
        let rules = plumb::depot::Rules::guard(self.path(), self.manifest.target())?;
        let product = crate::shape::product::at(root, "", &rules)?;
        if product.profile.is_none() {
            return Err("guard configuration requires a Product Profile".into());
        }
        Ok(product)
    }

    pub fn path(&self) -> &Path {
        self.temporary.path()
    }

    pub fn transaction(&self) -> bool {
        self.transaction
    }
}

pub(super) fn target(tree: &super::super::workflow::tree::Tree) -> Result<Option<String>, String> {
    let Some(governance) = tree.text("plumb.toml")? else {
        return Ok(None);
    };
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
    let text = tree
        .text("Cargo.toml")?
        .ok_or_else(|| "staged tree requires Cargo.toml".to_string())?;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Context {
    Line,
    Source,
}

impl Git<'_> {
    fn context(&self, version: &str) -> Result<Context, String> {
        let expected = format!("release/{version}");
        match self.run(&["symbolic-ref", "--short", "HEAD"], "read release line") {
            Ok(branch) if branch == expected => return Ok(Context::Line),
            Ok(branch) if branch.starts_with("release/") => {
                return Err(format!(
                    "configuration mismatch may bootstrap only on {expected}, got {branch}"
                ));
            }
            Ok(_) => return Ok(Context::Source),
            Err(_) => {
                let head = self.run(&["rev-parse", "HEAD"], "read detached release commit")?;
                if let Ok(remote) = self.run(
                    &["rev-parse", &format!("origin/{expected}^{{commit}}")],
                    "read remote release commit",
                ) && head == remote
                {
                    return Ok(Context::Line);
                }
            }
        }
        Ok(Context::Source)
    }

    fn commit(&self) -> Result<String, String> {
        self.run(&["rev-parse", "HEAD"], "read staged release commit")
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
