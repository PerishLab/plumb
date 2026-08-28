use super::module::{Module, channel, drift, integrity, publication, release};
use flate2::{Compression, GzBuilder, read::GzDecoder};
use semver::Version;
use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Request<'a> {
    pub package: &'a str,
    pub version: &'a str,
    pub credential: &'a str,
    pub reuse: &'a str,
}

pub fn run(carrier: &Module<'_>, request: Request<'_>) -> Result<String, String> {
    Exact { carrier }.run(request)
}

struct Exact<'a, 'b> {
    carrier: &'a Module<'b>,
}

struct Projection<'a> {
    npm: &'a crate::shape::release::Npm,
    package: &'a str,
    identity: &'a Version,
    token: &'a str,
    archive: &'a Path,
}

impl Exact<'_, '_> {
    fn run(&self, request: Request<'_>) -> Result<String, String> {
        let npm = self
            .carrier
            .spec
            .npm
            .as_ref()
            .ok_or_else(|| format!("{} has no module attachment", self.carrier.spec.product))?;
        if !npm.packages.iter().any(|held| held == request.package) {
            return Err(format!(
                "{} is not a declared module attachment",
                request.package
            ));
        }
        let source: Source = serde_json::from_str(request.reuse)
            .map_err(|error| format!("cannot parse --reuse: {error}"))?;
        source.validate()?;
        let identity = release(request.version)?;
        let archive = match source.kind.as_str() {
            "none" => self.package(request.package, &identity)?,
            "workload" => self.fetch(request.package, &identity, &source.source)?,
            "url" => return Err("a held publication URL must skip the module action".into()),
            _ => unreachable!(),
        };
        let token = crate::command::ship::attachment::credential(request.credential)?;
        let publication = self.publish(Projection {
            npm,
            package: request.package,
            identity: &identity,
            token,
            archive: &archive,
        })?;
        serde_json::to_string(&serde_json::json!({
            "format": "plumb.module-project/v1",
            "package": request.package,
            "version": request.version,
            "integrity": integrity(&archive)?,
            "workload": archive,
            "publication": publication,
        }))
        .map_err(|error| format!("cannot encode module project: {error}"))
    }

    fn package(&self, package: &str, identity: &Version) -> Result<PathBuf, String> {
        let manifest = self.carrier.seat(package).join("package.json");
        let original = std::fs::read(&manifest)
            .map_err(|error| format!("cannot read {}: {error}", manifest.display()))?;
        let restored = Scope::new(&manifest, &original);
        self.carrier.stamp(package, identity)?;
        let document: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&manifest)
                .map_err(|error| format!("cannot read {}: {error}", manifest.display()))?,
        )
        .map_err(|error| format!("cannot parse {}: {error}", manifest.display()))?;
        if document
            .pointer("/scripts/build")
            .and_then(serde_json::Value::as_str)
            .is_some()
        {
            self.carrier
                .pnpm(&["--filter", package, "build"], package)?;
        }
        let out = self.carrier.spec.root.join("target/module");
        std::fs::create_dir_all(&out)
            .map_err(|error| format!("cannot open {}: {error}", out.display()))?;
        let out = std::fs::canonicalize(&out)
            .map_err(|error| format!("cannot resolve {}: {error}", out.display()))?;
        self.carrier.pnpm(
            &["pack", "--pack-destination", &out.to_string_lossy()],
            package,
        )?;
        restored.restore()?;
        let archive = self.carrier.archive(package, identity);
        if archive.is_file() {
            Ok(archive)
        } else {
            Err(format!("module attachment left no {}", archive.display()))
        }
    }

    fn fetch(&self, package: &str, identity: &Version, source: &str) -> Result<PathBuf, String> {
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
            .map_err(|error| format!("cannot fetch reusable module workload {source}: {error}"))?;
        if !response.status.success() {
            return Err(format!("cannot fetch reusable module workload {source}"));
        }
        let archive = self.carrier.archive(package, identity);
        if let Some(parent) = archive.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot open {}: {error}", parent.display()))?;
        }
        repack(&response.stdout, &archive, package, identity)?;
        Ok(archive)
    }

    fn publish(&self, projection: Projection<'_>) -> Result<String, String> {
        let spec = format!("{}@{}", projection.package, projection.identity);
        let held = integrity(projection.archive)?;
        let seat = tempfile::tempdir()
            .map_err(|error| format!("cannot open a module projection seat: {error}"))?;
        if let Some(carried) =
            self.carrier
                .carried(projection.npm, &spec, projection.token, seat.path())?
        {
            drift(&spec, &carried, &held)?;
            return Ok(publication(projection.npm, projection.package));
        }
        let name = projection
            .archive
            .file_name()
            .ok_or_else(|| {
                format!(
                    "packed module has no name: {}",
                    projection.archive.display()
                )
            })?
            .to_string_lossy()
            .to_string();
        std::fs::copy(projection.archive, seat.path().join(&name))
            .map_err(|error| format!("cannot stage {}: {error}", projection.archive.display()))?;
        let mut command = vec![
            "publish",
            name.as_str(),
            "--registry",
            projection.npm.registry.as_str(),
        ];
        let tag = channel(projection.identity);
        if let Some(tag) = &tag {
            command.extend(["--tag", tag]);
        }
        self.carrier
            .authenticated(&command, projection.npm, projection.token, seat.path())?;
        let carried = self
            .carrier
            .carried(projection.npm, &spec, projection.token, seat.path())?
            .ok_or_else(|| format!("{spec} reports no integrity after publishing it"))?;
        drift(&spec, &carried, &held)?;
        Ok(publication(projection.npm, projection.package))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    #[serde(rename = "type")]
    kind: String,
    source: String,
}

impl Source {
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

struct Scope<'a> {
    path: &'a Path,
    original: &'a [u8],
    restored: bool,
}

impl<'a> Scope<'a> {
    fn new(path: &'a Path, original: &'a [u8]) -> Self {
        Self {
            path,
            original,
            restored: false,
        }
    }

    fn restore(mut self) -> Result<(), String> {
        std::fs::write(self.path, self.original)
            .map_err(|error| format!("cannot restore {}: {error}", self.path.display()))?;
        self.restored = true;
        Ok(())
    }
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        if !self.restored {
            let _ = std::fs::write(self.path, self.original);
        }
    }
}

fn repack(bytes: &[u8], output: &Path, package: &str, version: &Version) -> Result<(), String> {
    let mut source = tar::Archive::new(GzDecoder::new(bytes));
    let file = std::fs::File::create(output)
        .map_err(|error| format!("cannot create {}: {error}", output.display()))?;
    let encoder = GzBuilder::new()
        .mtime(0)
        .write(file, Compression::default());
    let mut target = tar::Builder::new(encoder);
    let mut manifest = false;
    for entry in source
        .entries()
        .map_err(|error| format!("cannot read reusable module workload: {error}"))?
    {
        let mut entry =
            entry.map_err(|error| format!("cannot read reusable module workload: {error}"))?;
        let path = entry
            .path()
            .map_err(|error| format!("cannot read reusable module path: {error}"))?
            .into_owned();
        let mut body = Vec::new();
        entry
            .read_to_end(&mut body)
            .map_err(|error| format!("cannot read reusable module entry: {error}"))?;
        let mut header = entry.header().clone();
        if path == Path::new("package/package.json") {
            let mut document: serde_json::Value = serde_json::from_slice(&body)
                .map_err(|error| format!("cannot parse reusable package manifest: {error}"))?;
            if document.get("name").and_then(serde_json::Value::as_str) != Some(package) {
                return Err(format!(
                    "reusable workload does not carry package {package}"
                ));
            }
            document["version"] = serde_json::Value::String(version.to_string());
            body = serde_json::to_vec(&document)
                .map_err(|error| format!("cannot encode reusable package manifest: {error}"))?;
            header.set_size(body.len() as u64);
            manifest = true;
        }
        header.set_mtime(0);
        header.set_cksum();
        target
            .append(&header, body.as_slice())
            .map_err(|error| format!("cannot write {}: {error}", output.display()))?;
    }
    if !manifest {
        return Err("reusable workload carries no package/package.json".into());
    }
    target
        .into_inner()
        .and_then(flate2::write::GzEncoder::finish)
        .map_err(|error| format!("cannot finish {}: {error}", output.display()))?;
    Ok(())
}
