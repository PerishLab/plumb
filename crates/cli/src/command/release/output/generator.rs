use crate::command::release::record::{Generator, Origin, Seal};

pub fn resolve(authority: &str) -> Result<Generator, String> {
    let running = plumb::version!("PLUMB").to_string();
    let source = option_env!("PLUMB_BUILD_SOURCE").and(plumb::commit!("PLUMB"));
    Ok(Generator {
        version: running.clone(),
        template: crate::command::release::manager::template()?,
        origin: Some(origin(authority, &running, source)?),
        recovery: None,
    })
}

fn origin(authority: &str, running: &str, source: Option<&str>) -> Result<Origin, String> {
    if let Some(commit) = source {
        super::super::proof::commit(commit)?;
        return Ok(Origin::Source {
            repository: env!("CARGO_PKG_REPOSITORY").into(),
            commit: commit.into(),
        });
    }
    let Some((_, pre)) = running.split_once('-') else {
        return Ok(Origin::Stable {});
    };
    let channel = pre
        .split('.')
        .next()
        .filter(|held| !held.is_empty())
        .ok_or_else(|| format!("generator version names no channel: {running}"))?
        .to_string();
    let version = running.to_string();
    let url = format!("{authority}/v1/releases/{channel}/{version}/seal.json");
    let sha256 = crate::command::release::verify::Surface(&url).digest()?;
    Ok(Origin::Exact {
        channel,
        version,
        url,
        sha256,
    })
}

pub fn audit(seal: &Seal) -> Result<(), String> {
    let Some(Origin::Exact {
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
