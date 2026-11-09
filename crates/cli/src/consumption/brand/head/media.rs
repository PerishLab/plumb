use std::collections::BTreeMap;

pub(super) fn link(attributes: &BTreeMap<String, String>) -> Result<(), String> {
    let rel = attributes
        .get("rel")
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    if ["manifest", "image_src"].contains(&rel.as_str())
        || rel.contains("icon")
            && !["icon", "shortcut icon", "apple-touch-icon"].contains(&rel.as_str())
    {
        return Err("additional head image/icon/manifest carriers are unqualified".into());
    }
    Ok(())
}

pub(super) fn meta(attributes: &BTreeMap<String, String>) -> Result<(), String> {
    if attributes.contains_key("name") && attributes.contains_key("property") {
        return Err("ambiguous head metadata name/property bindings are unqualified".into());
    }
    let key = attributes
        .get("property")
        .or_else(|| attributes.get("name"))
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    let families: [(&str, &[&str]); 3] = [
        (
            "og:image",
            &[
                "og:image",
                "og:image:alt",
                "og:image:width",
                "og:image:height",
            ],
        ),
        ("twitter:image", &["twitter:image", "twitter:image:alt"]),
        ("msapplication-", &[]),
    ];
    for (prefix, supported) in families {
        if key.starts_with(prefix) && !supported.contains(&key.as_str()) {
            return Err(
                "additional image metadata formats are outside the qualified head contract".into(),
            );
        }
    }
    Ok(())
}

pub(super) fn dimensions(
    found: &BTreeMap<String, String>,
    asset: &super::Asset,
) -> Result<(), String> {
    for (key, expected) in [
        ("og:image:width", asset.width),
        ("og:image:height", asset.height),
    ] {
        if let Some(value) = found.get(key)
            && value.parse::<u32>().ok() != Some(expected)
        {
            return Err("sharing metadata dimensions differ from the formal image".into());
        }
    }
    Ok(())
}
