use crate::command::release::record::{Origin, Seal};

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
