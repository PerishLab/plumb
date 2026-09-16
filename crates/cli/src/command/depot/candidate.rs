use plumb::depot::v3::{Bundle, Generation, Identity, Query};
use std::path::Path;
use std::sync::OnceLock;

static SELECTED: OnceLock<plumb::depot::v3::Pointer> = OnceLock::new();

#[derive(clap::Args)]
pub struct Selection {
    #[arg(long = "configuration-marker", requires = "generation", conflicts_with_all = ["base", "head", "write"])]
    pub marker: Option<String>,
    #[arg(
        long,
        requires = "marker",
        help = "Bootstrap with one installed marker-bound candidate generation"
    )]
    pub generation: Option<String>,
}

pub fn selected() -> Option<&'static plumb::depot::v3::Pointer> {
    SELECTED.get()
}

pub fn select(root: &Path, marker: &str, generation: &str) -> Result<(), String> {
    if plumb::config::value("PLUMB_HOME").is_none() {
        return Err("candidate Guard requires an explicit isolated PLUMB_HOME".into());
    }
    let seat = plumb::depot::root(Path::new(""))?;
    let bytes = std::fs::read(seat.join(plumb::depot::v3::POINTER))
        .map_err(|error| format!("cannot read installed candidate selection: {error}"))?;
    let pointer = plumb::depot::v3::Pointer::parse(&bytes)?;
    if pointer.version != marker || pointer.generation != generation {
        return Err("installed candidate does not match explicit marker and generation".into());
    }
    plumb::depot::candidate(&seat, marker)?;
    let held = crate::command::release::ReleaseMarker::bound(root, marker)?;
    if held.product != "plumb" || held.digest()? != pointer.marker.sha256 {
        return Err("candidate Guard configuration does not bind the release marker".into());
    }
    SELECTED
        .set(pointer)
        .map_err(|_| "candidate Guard was already selected".into())
}

pub fn restore(root: &Path) -> Result<(), String> {
    if plumb::config::value("PLUMB_HOME").is_none() {
        return Ok(());
    }
    let tree = plumb::guard::tree(root)?;
    let held = plumb::guard::inspect(root, &tree).or_else(|_| {
        let proof = plumb::guard::commit(root, "HEAD")?;
        proof.witness(root)?;
        Ok::<_, String>(proof)
    });
    let Some(evidence) = held.ok().and_then(|proof| proof.bootstrap) else {
        return Ok(());
    };
    evidence.current()?;
    select(root, &evidence.marker.name, &evidence.generation)
}

pub fn evidence(
    configuration: &plumb::guard::Configuration,
) -> Result<Option<plumb::guard::Bootstrap>, String> {
    let Some(pointer) = selected() else {
        return Ok(None);
    };
    let evidence = plumb::guard::Bootstrap {
        marker: pointer.marker.clone(),
        generation: pointer.generation.clone(),
        controller: plumb::guard::Bootstrap::executable()?,
        configuration: configuration.digest().to_string(),
        validator: configuration.validator().clone(),
    };
    evidence.validate()?;
    Ok(Some(evidence))
}

pub fn install(root: &Path, marker: &str, generation: &str, path: &Path) -> Result<String, String> {
    let marker = format!("v{}", marker.strip_prefix('v').unwrap_or(marker));
    let marker = marker.as_str();
    match std::fs::symlink_metadata(path) {
        Ok(_) => return Err("candidate configuration requires a new isolated --path".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(format!("cannot inspect candidate path: {error}")),
    }
    let rig = plumb::rig::Rig::resolve(None).map_err(|error| error.to_string())?;
    let channel = crate::command::release::channel(marker)?;
    let held = Generation::named(
        Query {
            source: &rig.rules.source,
            product: "plumb",
            channel: &channel,
            version: marker,
            kind: plumb::depot::v3::Kind::Configuration,
        },
        generation,
    )?;
    let bundle = super::contents(&held.manifest, |path| held.read(path))?;
    let target = crate::shape::product::review(root, &bundle.bodies)?;
    let spec = crate::shape::release::Spec::governed(root, target)?;
    let marker = crate::command::release::ReleaseMarker::configured(root, marker, spec.clone())?;
    if marker.product != "plumb" {
        return Err("plumb configuration requires a Plumb release marker".into());
    }
    let proof = marker.digest()?;
    let source = &marker
        .spec()
        .derivative(plumb::depot::v3::Kind::Configuration)?
        .source;
    let identity = plumb::depot::v3::Identity {
        product: marker.product.clone(),
        channel: marker.channel.clone(),
        version: marker.marker.clone(),
        marker: plumb::depot::v3::Marker {
            name: marker.marker.clone(),
            sha256: proof.clone(),
        },
        kind: plumb::depot::v3::Kind::Configuration,
    };
    if source.trim_end_matches('/') != rig.rules.source.trim_end_matches('/')
        || bundle.manifest.identity() != identity
    {
        return Err(
            "candidate generation does not bind the selected release marker and source".into(),
        );
    }
    if crate::command::release::ReleaseMarker::configured(root, &marker.marker, spec)?.digest()?
        != proof
    {
        return Err("release marker drifted while reading candidate configuration".into());
    }
    let pointer = plumb::depot::v3::Pointer::new(
        &bundle.manifest,
        plumb::depot::v3::Publication {
            source,
            prior: None,
            created: crate::command::clock::ahead(0)?,
        },
    )?;
    std::fs::create_dir(path)
        .map_err(|error| format!("cannot reserve isolated candidate path: {error}"))?;
    let installed = plumb::depot::v3::install(path, &pointer, &bundle)?;
    Ok(format!(
        "installed candidate configuration {} {generation} into {}; remote latest unchanged",
        marker.marker,
        installed.display()
    ))
}

pub fn read(source: &str, identity: Identity, generation: &str) -> Result<Bundle, String> {
    let held = Generation::named(
        Query {
            source,
            product: &identity.product,
            channel: &identity.channel,
            version: &identity.version,
            kind: identity.kind,
        },
        generation,
    )?;
    if held.manifest.identity() != identity {
        return Err("candidate generation does not bind the selected release marker".into());
    }
    super::contents(&held.manifest, |path| held.read(path))
}
