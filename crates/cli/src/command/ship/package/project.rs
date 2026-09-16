use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    #[serde(rename = "type")]
    pub kind: String,
    pub source: String,
}

impl Source {
    pub fn parse(text: &str) -> Result<Self, String> {
        let held: Self =
            serde_json::from_str(text).map_err(|error| format!("cannot parse --reuse: {error}"))?;
        held.validate()?;
        Ok(held)
    }

    fn validate(&self) -> Result<(), String> {
        match (self.kind.as_str(), self.source.as_str()) {
            ("none", "") => Ok(()),
            ("workload" | "url", source) if source.starts_with("https://") => Ok(()),
            ("none" | "workload" | "url", _) => {
                Err("--reuse source does not match its type".into())
            }
            _ => Err("--reuse type must be none, workload, or url".into()),
        }
    }
}

pub fn fetch(kind: &str, source: &str) -> Result<Vec<u8>, String> {
    use sha2::{Digest, Sha256};
    let expected = source
        .split_once("/v2/blobs/sha256/")
        .map(|(_, digest)| digest)
        .filter(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
        .ok_or("reusable workload requires a content-addressed blob URL")?;
    let response = Command::new("curl")
        .args([
            "--fail-with-body",
            "--silent",
            "--show-error",
            "--location",
            "--retry",
            "3",
            source,
        ])
        .output()
        .map_err(|error| format!("cannot fetch reusable {kind} workload {source}: {error}"))?;
    if response.status.success() {
        if format!("{:x}", Sha256::digest(&response.stdout)) != expected {
            return Err(format!("reusable {kind} workload digest mismatch"));
        }
        Ok(response.stdout)
    } else {
        Err(format!("cannot fetch reusable {kind} workload {source}"))
    }
}

pub struct Scope {
    path: PathBuf,
    original: Vec<u8>,
    restored: bool,
}

impl Scope {
    pub fn read(path: &Path) -> Result<Self, String> {
        let original = std::fs::read(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            original,
            restored: false,
        })
    }

    pub fn restore(mut self) -> Result<(), String> {
        std::fs::write(&self.path, &self.original)
            .map_err(|error| format!("cannot restore {}: {error}", self.path.display()))?;
        self.restored = true;
        Ok(())
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if !self.restored {
            let _ = std::fs::write(&self.path, &self.original);
        }
    }
}

pub fn cargo(
    carrier: &crate::command::ship::adaptor::registry::Registry<'_>,
    version: &str,
    token: &str,
    reuse: &str,
) -> Result<String, String> {
    let cargo = carrier
        .spec
        .cargo
        .as_ref()
        .ok_or_else(|| format!("{} has no Cargo attachment", carrier.spec.product))?;
    if token.trim().is_empty() {
        return Err("PLUMB_RELEASE_REGISTRY_TOKEN is required".into());
    }
    let source = Source::parse(reuse)?;
    if source.kind != "workload" {
        return Err("Cargo publication requires its prepared source workload".into());
    }
    let seat = tempfile::Builder::new()
        .prefix("cargo-publication-")
        .tempdir()
        .map_err(|error| error.to_string())?
        .keep();
    restore(&fetch("Cargo", &source.source)?, &seat)?;
    let mut spec = carrier.spec.clone();
    spec.root = seat;
    let carrier = crate::command::ship::adaptor::registry::registry(&spec);
    carrier.prepare(version)?;
    let identity = crate::command::release::workspace::release(version)?;
    let mut members = BTreeMap::new();
    for package in carrier.ordered(cargo)? {
        carrier.command(
            [
                "package",
                "--registry",
                &cargo.registry,
                "--package",
                package,
                "--allow-dirty",
                "--no-verify",
            ],
            token,
        )?;
        let archive = carrier.archive(package, &identity);
        crate::command::ship::adaptor::registry::inspect(&archive, package, &identity)?;
        let bytes = std::fs::read(&archive)
            .map_err(|error| format!("cannot read {}: {error}", archive.display()))?;
        publish(Publication {
            carrier: &carrier,
            cargo,
            package,
            identity: &identity,
            archive: &archive,
            token,
        })?;
        members.insert(
            PathBuf::from(format!("{package}-{identity}.crate")),
            crate::command::ship::archive::Member { bytes, mode: 0o644 },
        );
    }
    let seat = carrier.spec.root.join("target/cargo");
    std::fs::create_dir_all(&seat)
        .map_err(|error| format!("cannot open {}: {error}", seat.display()))?;
    let path = seat.join(format!("{}-cargo.tar.gz", carrier.spec.product));
    crate::command::ship::archive::bundle(&path, &members)?;
    let package = carrier
        .ordered(cargo)?
        .last()
        .ok_or_else(|| "Cargo attachment must declare ordered packages".to_string())?;
    serde_json::to_string(&serde_json::json!({
        "format": "plumb.cargo-project/v1",
        "version": version,
        "packages": cargo.packages,
        "workload": path,
        "publication": crate::command::ship::adaptor::ledger::publication(
            cargo,
            package,
        )?,
    }))
    .map_err(|error| format!("cannot encode Cargo project: {error}"))
}

struct Publication<'a, 'b> {
    carrier: &'a crate::command::ship::adaptor::registry::Registry<'b>,
    cargo: &'a crate::shape::release::Cargo,
    package: &'a str,
    identity: &'a semver::Version,
    archive: &'a Path,
    token: &'a str,
}

fn publish(input: Publication<'_, '_>) -> Result<(), String> {
    let checksum = crate::command::release::record::digest(input.archive)?.0;
    if crate::command::ship::adaptor::ledger::verified(
        input.package,
        input.identity,
        &checksum,
        &crate::command::ship::adaptor::ledger::entries(input.cargo, input.package, input.token)?,
    )? {
        return Ok(());
    }
    input.carrier.command(
        [
            "publish",
            "--registry",
            &input.cargo.registry,
            "--package",
            input.package,
            "--allow-dirty",
            "--no-verify",
        ],
        input.token,
    )?;
    crate::command::ship::adaptor::ledger::readback(
        crate::command::ship::adaptor::ledger::Readback {
            cargo: input.cargo,
            package: input.package,
            version: input.identity,
            checksum: &checksum,
            token: input.token,
        },
    )
}

fn restore(bytes: &[u8], root: &Path) -> Result<(), String> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    let mut found = std::collections::BTreeSet::new();
    for entry in archive.entries().map_err(|error| error.to_string())? {
        let mut entry = entry.map_err(|error| error.to_string())?;
        let path = entry
            .path()
            .map_err(|error| error.to_string())?
            .into_owned();
        if !entry.header().entry_type().is_file() || !relative(&path) || !found.insert(path.clone())
        {
            return Err("Cargo source workload has an invalid or repeated path".into());
        }
        if !entry.unpack_in(root).map_err(|error| error.to_string())? {
            return Err("Cargo source workload escapes its materialization root".into());
        }
    }
    if !root.join("Cargo.toml").is_file() {
        return Err("Cargo source workload has no native manifest".into());
    }
    Ok(())
}

fn relative(path: &Path) -> bool {
    path.components()
        .all(|part| matches!(part, std::path::Component::Normal(_)))
        && !path.components().any(|part| part.as_os_str() == ".git")
}
