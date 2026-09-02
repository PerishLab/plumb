use crate::command::release;

pub(super) fn held(marker: &release::ReleaseMarker) -> Result<bool, String> {
    let url = format!(
        "{}/v1/releases/{}/{}/seal.json",
        marker.authority, marker.channel, marker.marker
    );
    let Some(seal) = release::verify::optional(&url)? else {
        return Ok(false);
    };
    let actual = (
        seal["schema"].as_u64(),
        seal["product"].as_str(),
        seal["channel"].as_str(),
        seal["releaseVersion"].as_str(),
        seal["commit"].as_str(),
        seal["url"].as_str(),
    );
    let expected = (
        Some(1),
        Some(marker.product.as_str()),
        Some(marker.channel.as_str()),
        Some(marker.marker.as_str()),
        Some(marker.commit.as_str()),
        Some(url.as_str()),
    );
    if actual == expected {
        Ok(true)
    } else {
        Err(format!(
            "release marker {} disagrees with its published binary seal",
            marker.marker
        ))
    }
}
