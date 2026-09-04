use super::adaptor;
use crate::command::release::{artifacts, capsule, required, verify};
use plumb::rig::Rig;

pub(super) fn cargo(deed: super::Cargo) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = crate::shape::release::Spec::controller(&rig.release.root)?;
    let release = &rig.release;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    let attachment = adaptor::registry::registry(&spec);
    match deed {
        super::Cargo::Publish => {
            sealed(&spec, release, version, spec.cargo.is_some())?;
            attachment.publish(version, &release.credential)
        }
        super::Cargo::Rehearse => attachment.rehearse(version, &release.credential),
    }
}

pub(super) fn oci(deed: super::Oci) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = crate::shape::release::Spec::controller(&rig.release.root)?;
    let release = &rig.release;
    let carrier = adaptor::image::image(&spec);
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    match deed {
        super::Oci::Exact { reuse } => {
            sealed(&spec, release, version, spec.oci.is_some())?;
            adaptor::container::run(
                &carrier,
                adaptor::container::Request {
                    version,
                    commit: &release.commit,
                    artifacts: &artifacts(release)?,
                    credential: &release.credential,
                    reuse: &reuse,
                },
            )
        }
        super::Oci::Build => carrier.build(version, &release.commit, &artifacts(release)?),
        super::Oci::Publish => {
            sealed(&spec, release, version, spec.oci.is_some())?;
            carrier.publish(version, &release.credential)
        }
    }
}

pub(super) fn chart(deed: super::Chart) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = crate::shape::release::Spec::controller(&rig.release.root)?;
    let release = &rig.release;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    let carrier = adaptor::chart::chart(&spec);
    match deed {
        super::Chart::Exact { reuse } => {
            sealed(&spec, release, version, spec.chart.is_some())?;
            carrier.exact(version, &release.credential, &reuse)
        }
        super::Chart::Package => carrier.package(version),
        super::Chart::Publish => {
            sealed(&spec, release, version, spec.chart.is_some())?;
            carrier.publish(version, &release.credential)
        }
    }
}

pub(super) fn npm(deed: super::Npm) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = crate::shape::release::Spec::controller(&rig.release.root)?;
    let release = &rig.release;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    let carrier = adaptor::module::module(&spec);
    match deed {
        super::Npm::Exact { package, reuse } => {
            sealed(&spec, release, version, spec.npm.is_some())?;
            carrier.exact(&package, version, &release.credential, &reuse)
        }
        super::Npm::Pack => carrier.pack(version),
        super::Npm::Publish => {
            sealed(&spec, release, version, spec.npm.is_some())?;
            carrier.publish(version, &release.credential)
        }
    }
}

pub(in crate::command::ship) fn credential(credential: &str) -> Result<&str, String> {
    let credential = required("PLUMB_RELEASE_REGISTRY_TOKEN", credential)?;
    credential
        .strip_prefix("Bearer ")
        .filter(|token| !token.chars().any(char::is_whitespace))
        .filter(|token| !token.is_empty())
        .ok_or_else(|| "PLUMB_RELEASE_REGISTRY_TOKEN must be a Cargo Bearer credential".into())
}

pub(in crate::command::ship) struct Identity<'a> {
    pub user: &'a str,
    pub token: &'a str,
}

pub(super) fn sealed(
    spec: &crate::shape::release::Spec,
    release: &plumb::rig::Release,
    version: &str,
    declared: bool,
) -> Result<(), String> {
    if !spec.binary() {
        return Ok(());
    }
    let path = capsule(release)?;
    let (compiled, _) = crate::command::release::record::Capsule::read(&path)?;
    if compiled.version != version {
        return Err(format!(
            "capsule seals {} while the projection carries {version}",
            compiled.version
        ));
    }
    if !declared {
        return Ok(());
    }
    verify::object(&compiled.seal)
}
