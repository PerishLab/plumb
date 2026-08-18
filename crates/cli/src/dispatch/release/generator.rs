use super::model::Spec;
use super::record::{Generator, GeneratorOrigin, RecoveryIdentity, Seal, sha};
use std::path::Path;

pub const PRODUCT: &str = "plumb";
pub const AUTHORITY: &str = "https://releases.plumb.perish.uk";
pub const REPOSITORY: &str = "PerishLab/plumb";
pub const BETA_CHANNEL: &str = "beta";
pub const BETA_VERSION: &str = "v0.26.0-beta.5";
pub const STABLE_VERSION: &str = "v0.26.0";

pub struct Claim<'a> {
    pub spec: &'a Spec,
    pub channel: &'a str,
    pub version: &'a str,
    pub commit: &'a str,
    pub promotion: Option<&'a Path>,
}

#[derive(Clone, Copy)]
struct Running<'a> {
    version: &'a str,
}

pub fn contract(spec: &Spec) -> Result<(), String> {
    if spec.product != PRODUCT || spec.authority != AUTHORITY {
        return Err(format!(
            "the one-time bootstrap belongs only to {REPOSITORY} at {AUTHORITY}"
        ));
    }
    Ok(())
}

pub fn resolve(input: Claim<'_>) -> Result<Generator, String> {
    resolve_with(
        input,
        Running {
            version: plumb::version!("PLUMB"),
        },
    )
}

fn resolve_with(input: Claim<'_>, running: Running<'_>) -> Result<Generator, String> {
    let origin = if bootstrap(&input, running) {
        contract(input.spec)?;
        exact(&input, running)?
    } else {
        GeneratorOrigin::Stable {}
    };
    Ok(Generator {
        version: running.version.into(),
        template: super::manager::template(),
        origin: Some(origin),
        recovery: bootstrap(&input, running).then(identity),
    })
}

fn bootstrap(input: &Claim<'_>, running: Running<'_>) -> bool {
    input.spec.product == PRODUCT
        && input.channel == "stable"
        && input.version == STABLE_VERSION
        && running.version == BETA_VERSION
}

fn identity() -> RecoveryIdentity {
    RecoveryIdentity {
        repository: REPOSITORY.into(),
        authority: AUTHORITY.into(),
        beta: BETA_VERSION.into(),
        stable: STABLE_VERSION.into(),
    }
}

fn exact_identity(generator: &Generator) -> bool {
    generator.recovery.as_ref() == Some(&identity())
}

fn exact(input: &Claim<'_>, running: Running<'_>) -> Result<GeneratorOrigin, String> {
    if running.version != BETA_VERSION {
        return Err(format!(
            "stable {STABLE_VERSION} requires generator {BETA_VERSION}, got {}",
            running.version
        ));
    }
    let path = input
        .promotion
        .ok_or_else(|| format!("stable {STABLE_VERSION} requires the exact public beta seal"))?;
    let bytes = std::fs::read(path).map_err(|error| {
        format!(
            "cannot read bootstrap beta seal {}: {error}",
            path.display()
        )
    })?;
    let seal: Seal = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "cannot parse bootstrap beta seal {}: {error}",
            path.display()
        )
    })?;
    beta(&seal, input.commit)?;
    Ok(GeneratorOrigin::ExactRelease {
        channel: BETA_CHANNEL.into(),
        version: BETA_VERSION.into(),
        url: seal.url,
        sha256: sha(&bytes),
    })
}

fn beta(seal: &Seal, commit: &str) -> Result<(), String> {
    let identity = seal.schema == 1
        && seal.product == PRODUCT
        && seal.channel == BETA_CHANNEL
        && seal.version == BETA_VERSION
        && seal.commit == commit
        && seal.url == format!("{AUTHORITY}/v1/releases/{BETA_CHANNEL}/{BETA_VERSION}/seal.json");
    if !identity {
        return Err("bootstrap beta seal identity disagrees".into());
    }
    Ok(())
}

pub fn audit(seal: &Seal) -> Result<(), String> {
    if seal.product != PRODUCT {
        return Ok(());
    }
    match (seal.channel.as_str(), seal.version.as_str()) {
        ("stable", STABLE_VERSION) => stable(seal),
        _ => {
            if matches!(
                seal.generator.origin,
                Some(GeneratorOrigin::SourceBuilt { .. } | GeneratorOrigin::ExactRelease { .. })
            ) {
                Err("bootstrap generator provenance appears outside its one release".into())
            } else {
                Ok(())
            }
        }
    }
}

fn stable(seal: &Seal) -> Result<(), String> {
    if !exact_identity(&seal.generator) {
        return Err("bootstrap generator identity disagrees".into());
    }
    let Some(proof) = &seal.proof else {
        return Err("bootstrap stable seal has no beta proof".into());
    };
    beta(&proof.seal, &seal.commit)?;
    match &seal.generator.origin {
        Some(GeneratorOrigin::ExactRelease {
            channel,
            version,
            url,
            sha256,
        }) if seal.generator.version == BETA_VERSION
            && channel == BETA_CHANNEL
            && version == BETA_VERSION
            && url == &proof.seal.url
            && sha256 == &proof.digest =>
        {
            Ok(())
        }
        _ => Err("bootstrap generator and beta promotion proof disagree".into()),
    }
}
