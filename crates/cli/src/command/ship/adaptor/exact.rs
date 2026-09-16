use super::module::{Module, channel, drift, integrity, publication, release};
use super::projection::Workload;
use crate::command::ship::package::project::{Scope, Source, fetch};
use semver::Version;
use std::path::{Path, PathBuf};

pub struct Request<'a> {
    pub package: &'a str,
    pub version: &'a str,
    pub credential: &'a str,
    pub reuse: &'a str,
}

pub fn run(carrier: &Module<'_>, request: Request<'_>) -> Result<String, String> {
    Exact { carrier }.run(request)
}

pub(super) fn produce(carrier: &Module<'_>, package: &str) -> Result<PathBuf, String> {
    Exact { carrier }.package(package, &Version::new(0, 0, 0))
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
        let source = Source::parse(request.reuse)?;
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
        let restored = Scope::read(&manifest)?;
        self.carrier.stamp(package, identity)?;
        let document: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&manifest)
                .map_err(|error| format!("cannot read {}: {error}", manifest.display()))?,
        )
        .map_err(|error| format!("cannot parse {}: {error}", manifest.display()))?;
        if document
            .pointer("/scripts/prepack")
            .and_then(serde_json::Value::as_str)
            .is_none()
            && document
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
        let response = fetch("module", source)?;
        let archive = self.carrier.archive(package, identity);
        if let Some(parent) = archive.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot open {}: {error}", parent.display()))?;
        }
        Workload::new(&response).bind(&archive, package, identity)?;
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
            "--no-git-checks",
            "--ignore-scripts",
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
