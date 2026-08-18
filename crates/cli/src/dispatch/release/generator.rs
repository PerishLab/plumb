use super::record::{Generator, GeneratorOrigin, RecoveryIdentity, Seal};

pub const PRODUCT: &str = "plumb";
pub const AUTHORITY: &str = "https://releases.plumb.perish.uk";
pub const REPOSITORY: &str = "PerishLab/plumb";
pub const BETA_CHANNEL: &str = "beta";
pub const BETA_VERSION: &str = "v0.26.0-beta.5";
pub const STABLE_VERSION: &str = "v0.26.0";

pub fn resolve() -> Result<Generator, String> {
    Ok(Generator {
        version: plumb::version!("PLUMB").into(),
        template: super::manager::template(),
        origin: Some(GeneratorOrigin::Stable {}),
        recovery: None,
    })
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
