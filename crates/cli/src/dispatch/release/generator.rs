use super::model::Spec;
use super::record::{Generator, GeneratorOrigin, RecoveryIdentity, Seal, sha};
use std::path::Path;

pub const PRODUCT: &str = "plumb";
pub const AUTHORITY: &str = "https://releases.plumb.perish.uk";
pub const REPOSITORY: &str = "PerishLab/plumb";
pub const BETA_CHANNEL: &str = "beta";
pub const BETA_VERSION: &str = "v0.18.14-beta.1";
pub const STABLE_VERSION: &str = "v0.18.14";
pub const RELEASE_BRANCH: &str = "release/v0.18.14";
pub const WORKFLOW: &str = "release-recovery.yml";

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
    commit: Option<&'a str>,
}

pub fn contract(spec: &Spec) -> Result<(), String> {
    if spec.product != PRODUCT || spec.authority != AUTHORITY {
        return Err(format!(
            "self-hosting recovery belongs only to {REPOSITORY} at {AUTHORITY}"
        ));
    }
    Ok(())
}

pub fn resolve(input: Claim<'_>) -> Result<Generator, String> {
    resolve_with(
        input,
        Running {
            version: plumb::version!("PLUMB"),
            commit: plumb::commit!("PLUMB"),
        },
    )
}

fn resolve_with(input: Claim<'_>, running: Running<'_>) -> Result<Generator, String> {
    let origin = if recovery(&input) {
        contract(input.spec)?;
        if input.channel == BETA_CHANNEL {
            source(&input, running)?
        } else {
            exact(&input, running)?
        }
    } else {
        GeneratorOrigin::Stable {}
    };
    Ok(Generator {
        version: running.version.into(),
        template: super::manager::template(),
        origin: Some(origin),
        recovery: recovery(&input).then(identity),
    })
}

fn recovery(input: &Claim<'_>) -> bool {
    let running = plumb::version!("PLUMB");
    input.spec.product == PRODUCT
        && ((input.channel == BETA_CHANNEL
            && input.version == BETA_VERSION
            && running == STABLE_VERSION)
            || (input.channel == "stable"
                && input.version == STABLE_VERSION
                && running == BETA_VERSION))
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

fn source(input: &Claim<'_>, running: Running<'_>) -> Result<GeneratorOrigin, String> {
    if input.promotion.is_some() {
        return Err("the recovery beta cannot carry a promotion proof".into());
    }
    if running.version != STABLE_VERSION {
        return Err(format!(
            "the recovery beta requires source Plumb {STABLE_VERSION}, got {}",
            running.version
        ));
    }
    let commit = running
        .commit
        .ok_or_else(|| "the recovery beta generator has no compiled source commit".to_string())?;
    super::proof::commit(commit)
        .map_err(|_| "the recovery beta generator source commit is invalid".to_string())?;
    Ok(GeneratorOrigin::SourceBuilt {
        repository: REPOSITORY.into(),
        commit: commit.into(),
    })
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
    let bytes = std::fs::read(path)
        .map_err(|error| format!("cannot read recovery beta seal {}: {error}", path.display()))?;
    let seal: Seal = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "cannot parse recovery beta seal {}: {error}",
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
    let source = exact_identity(&seal.generator)
        && matches!(
            &seal.generator.origin,
            Some(GeneratorOrigin::SourceBuilt { repository, commit })
                if repository == REPOSITORY
                    && seal.generator.version == STABLE_VERSION
                    && super::proof::commit(commit).is_ok()
        );
    if !identity || !source {
        return Err("exact recovery beta seal identity or source provenance disagrees".into());
    }
    Ok(())
}

pub fn audit(seal: &Seal) -> Result<(), String> {
    if seal.product != PRODUCT {
        return Ok(());
    }
    match (seal.channel.as_str(), seal.version.as_str()) {
        (BETA_CHANNEL, BETA_VERSION) => beta(seal, &seal.commit),
        ("stable", STABLE_VERSION) => stable(seal),
        _ => {
            if matches!(
                seal.generator.origin,
                Some(GeneratorOrigin::SourceBuilt { .. } | GeneratorOrigin::ExactRelease { .. })
            ) {
                Err("self-hosting generator provenance appears outside its exact recovery".into())
            } else {
                Ok(())
            }
        }
    }
}

fn stable(seal: &Seal) -> Result<(), String> {
    if !exact_identity(&seal.generator) {
        return Err("stable recovery generator identity disagrees".into());
    }
    let Some(proof) = &seal.proof else {
        return Err("stable recovery seal has no beta proof".into());
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
        _ => Err("stable recovery generator and beta promotion proof disagree".into()),
    }
}
