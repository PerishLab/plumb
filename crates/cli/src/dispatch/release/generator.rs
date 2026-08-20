use super::record::{Generator, GeneratorOrigin, Seal};

pub fn resolve(authority: &str) -> Result<Generator, String> {
    let running = plumb::version!("PLUMB").to_string();
    Ok(Generator {
        version: running.clone(),
        template: super::manager::template()?,
        origin: Some(origin(authority, &running)?),
        recovery: None,
    })
}

fn origin(authority: &str, running: &str) -> Result<GeneratorOrigin, String> {
    let Some((_, pre)) = running.split_once('-') else {
        return Ok(GeneratorOrigin::Stable {});
    };
    let channel = pre
        .split('.')
        .next()
        .filter(|held| !held.is_empty())
        .ok_or_else(|| format!("generator version names no channel: {running}"))?
        .to_string();
    let version = running.to_string();
    let url = format!("{authority}/v1/releases/{channel}/{version}/seal.json");
    let sha256 = super::verify::Surface(&url).digest()?;
    Ok(GeneratorOrigin::ExactRelease {
        channel,
        version,
        url,
        sha256,
    })
}

pub fn audit(seal: &Seal) -> Result<(), String> {
    let Some(GeneratorOrigin::ExactRelease {
        channel,
        version,
        url,
        ..
    }) = &seal.generator.origin
    else {
        return Ok(());
    };
    if line(version) != line(&seal.version) {
        return Err(format!(
            "generator {version} is not a point on the line releasing {}",
            seal.version
        ));
    }
    let expected = format!("v1/releases/{channel}/{version}/seal.json");
    if !url.ends_with(&expected) {
        return Err(format!("generator {version} names {url}, not {expected}"));
    }
    Ok(())
}

fn line(version: &str) -> &str {
    version.split_once('-').map_or(version, |(held, _)| held)
}
