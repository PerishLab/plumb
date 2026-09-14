mod archive;
pub(in crate::command::ship) mod proof;
mod sign;
pub(in crate::command::ship) mod workload;

use crate::command::release::{self, artifacts, required};
use crate::shape::release::Spec;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Build {
    reuse: crate::command::ship::package::project::Source,
    keys: serde_json::Value,
    production: String,
    receipt: Option<plumb::rule::Receipt>,
}

pub(super) struct Request<'a> {
    pub spec: &'a Spec,
    pub release: &'a plumb::rig::Release,
    pub action: &'a str,
    pub target: &'a str,
    pub archive: &'a str,
    pub build: Build,
}

pub(super) fn run(request: Request<'_>) -> Result<(PathBuf, plumb::rule::Receipt), String> {
    request.run()
}

pub(super) fn verify(
    spec: &Spec,
    marker: &release::ReleaseMarker,
    path: &Path,
    triple: &str,
) -> Result<(), String> {
    let target = spec.target(triple)?;
    let digest = marker.digest()?;
    for name in &spec.binaries {
        let filename = if matches!(target.format, crate::shape::release::Format::Zip) {
            format!("{name}.exe")
        } else {
            name.clone()
        };
        let bytes = archive::read(target.format, path, &filename)?;
        let (origin, identity) = plumb::identity::inspect(&bytes)?;
        let identity = identity.ok_or("publication carries an unbound executable")?;
        let actual = (
            &identity.product,
            &identity.marker,
            &identity.commit,
            &identity.digest,
        );
        let expected = (&marker.product, &marker.marker, &marker.commit, &digest);
        if origin.target != triple || actual != expected {
            return Err("publication binary identity differs from its release marker".into());
        }
    }
    Ok(())
}

impl Request<'_> {
    fn run(self) -> Result<(PathBuf, plumb::rule::Receipt), String> {
        let target = self.spec.target(self.target)?;
        if target.archive != self.archive {
            return Err("identity request archive differs from its target".into());
        }
        let version = required("PLUMB_RELEASE_VERSION", &self.release.version)?;
        let marker = release::snapshot(version)?;
        if marker.product != self.spec.product || marker.commit != self.release.commit {
            return Err("identity request differs from its release marker".into());
        }
        let before = marker.digest()?;
        let temporary = tempfile::tempdir().map_err(|error| error.to_string())?;
        let original = self.content(&marker, temporary.path())?;
        let contract = proof::contract(&marker, self.target)?;
        let producer = contract.start(&self.spec.root)?;
        let digest = release::record::digest(&original)?.0;
        let binding = plumb::identity::Binding {
            product: marker.product.clone(),
            marker: marker.marker.clone(),
            digest: before.clone(),
            commit: marker.commit.clone(),
            workload: digest,
        };
        let directory = temporary.path().join("bound");
        fs::create_dir(&directory).map_err(|error| error.to_string())?;
        let mut binaries = BTreeMap::new();
        for name in &self.spec.binaries {
            let suffix = if matches!(target.format, crate::shape::release::Format::Zip) {
                ".exe"
            } else {
                ""
            };
            let filename = format!("{name}{suffix}");
            let bytes = archive::read(target.format, &original, &filename)?;
            let (origin, held) = plumb::identity::inspect(&bytes)?;
            if origin.target != self.target || origin.commit.is_empty() || held.is_some() {
                return Err("reusable binary has no matching unbound build provenance".into());
            }
            let bound = plumb::identity::bind(&bytes, &binding)?;
            let path = directory.join(&filename);
            fs::write(&path, &bound).map_err(|error| error.to_string())?;
            sign::finalize(&path, self.target)?;
            let finished = fs::read(&path).map_err(|error| error.to_string())?;
            if plumb::identity::inspect(&finished)? != (origin, Some(binding.clone())) {
                return Err("platform finalization changed executable identity".into());
            }
            sign::probe(&path, name, &marker.marker)?;
            binaries.insert(name.clone(), path);
        }
        let output = artifacts(self.release)?;
        fs::create_dir_all(&output).map_err(|error| error.to_string())?;
        let output = output.join(self.archive);
        if output.exists() {
            return Err("bound artifact output already exists".into());
        }
        crate::command::ship::archive::write(target.format, &output, &binaries)?;
        if release::snapshot(version)?.digest()? != before {
            return Err("release marker drifted during executable binding".into());
        }
        let receipt = producer.finish(&output)?;
        contract.verify(&receipt)?;
        Ok((output, receipt))
    }

    fn content(
        &self,
        marker: &release::ReleaseMarker,
        temporary: &Path,
    ) -> Result<PathBuf, String> {
        let contract = super::transport::production::contract(marker, self.target)?;
        if self.build.production != contract.digest()? {
            return Err("reusable production contract differs from its marker plan".into());
        }
        let path = temporary.join(self.archive);
        match self.build.reuse.kind.as_str() {
            "none" if self.build.reuse.source.is_empty() => {
                let receipt =
                    super::transport::production::produce(marker, self.target, temporary)?;
                crate::command::workflow::record::project(
                    crate::command::workflow::record::Project {
                        action: self.action,
                        keys: &self.build.keys.to_string(),
                        workload: path.clone(),
                        reuse: None,
                        publication: None,
                        depot: None,
                        production: Some((&contract, receipt)),
                    },
                )?;
            }
            "workload" => {
                let receipt = self
                    .build
                    .receipt
                    .as_ref()
                    .ok_or("reused binary has no production receipt")?;
                contract.verify(receipt)?;
                let source = &self.build.reuse.source;
                let digest = source
                    .rsplit('/')
                    .next()
                    .and_then(|name| name.strip_suffix(".tgz"))
                    .filter(|value| {
                        value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
                    })
                    .ok_or("reusable binary URL carries no digest")?;
                if !source.starts_with("https://") {
                    return Err("reusable binary URL requires HTTPS".into());
                }
                let bytes = crate::command::ship::package::project::fetch("binary", source)?;
                if release::record::sha(&bytes) != digest {
                    return Err("reusable binary digest mismatch".into());
                }
                fs::write(&path, bytes).map_err(|error| error.to_string())?;
                receipt.verify(&path)?;
                println!("reused unbound binary workload for {}", self.target);
            }
            _ => {
                return Err("identity binding requires an unbound workload or a cold build".into());
            }
        }
        Ok(path)
    }
}
