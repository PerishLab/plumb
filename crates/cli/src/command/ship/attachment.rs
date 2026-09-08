use super::adaptor;
use crate::command::release::{artifacts, required};
use plumb::rig::Rig;

pub(super) fn cargo(deed: super::Cargo) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = crate::shape::release::Spec::controller(&rig.release.root)?;
    let release = &rig.release;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    let attachment = adaptor::registry::registry(&spec);
    match deed {
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
        super::Oci::Build => carrier.build(version, &release.commit, &artifacts(release)?),
    }
}

pub(super) fn chart(deed: super::Chart) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = crate::shape::release::Spec::controller(&rig.release.root)?;
    let release = &rig.release;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    let carrier = adaptor::chart::chart(&spec);
    match deed {
        super::Chart::Package => carrier.package(version),
    }
}

pub(super) fn npm(deed: super::Npm) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = crate::shape::release::Spec::controller(&rig.release.root)?;
    let release = &rig.release;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    let carrier = adaptor::module::module(&spec);
    match deed {
        super::Npm::Pack => carrier.pack(version),
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
