use super::adaptor;
use crate::command::release::{artifacts, capsule, required, verify};
use plumb::rig::Rig;

pub(super) fn cargo(deed: super::Cargo) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = crate::shape::release::Spec::read(&rig.release.root.join("plumb.toml"))?;
    let release = &rig.release;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    let attachment = adaptor::registry::registry(&spec);
    match deed {
        super::Cargo::Publish => {
            sealed(&spec, release, version, spec.cargo.is_some())?;
            attachment.publish(version, &release.registry_token)
        }
        super::Cargo::Rehearse => attachment.rehearse(version, &release.registry_token),
    }
}

pub(super) fn oci(deed: super::Oci) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = crate::shape::release::Spec::read(&rig.release.root.join("plumb.toml"))?;
    let release = &rig.release;
    let carrier = adaptor::image::image(&spec);
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    match deed {
        super::Oci::Build => carrier.build(version, &release.commit, &artifacts(release)?),
        super::Oci::Publish => {
            sealed(&spec, release, version, spec.oci.is_some())?;
            carrier.publish(version, &release.registry_token)
        }
    }
}

pub(super) fn chart(deed: super::Chart) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = crate::shape::release::Spec::read(&rig.release.root.join("plumb.toml"))?;
    let release = &rig.release;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    let carrier = adaptor::chart::chart(&spec);
    match deed {
        super::Chart::Package => carrier.package(version),
        super::Chart::Publish => {
            sealed(&spec, release, version, spec.chart.is_some())?;
            carrier.publish(version, &release.registry_token)
        }
    }
}

pub(super) fn npm(deed: super::Npm) -> Result<String, String> {
    let rig = Rig::resolve(None).map_err(|error| error.to_string())?;
    let spec = crate::shape::release::Spec::read(&rig.release.root.join("plumb.toml"))?;
    let release = &rig.release;
    let version = required("PLUMB_RELEASE_VERSION", &release.version)?;
    let carrier = adaptor::module::module(&spec);
    match deed {
        super::Npm::Pack => carrier.pack(version),
        super::Npm::Publish => {
            sealed(&spec, release, version, spec.npm.is_some())?;
            carrier.publish(version, &release.registry_token)
        }
    }
}

pub(in crate::command::ship) fn registry_token(credential: &str) -> Result<&str, String> {
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

fn sealed(
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
