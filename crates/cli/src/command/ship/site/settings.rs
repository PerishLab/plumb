use plumb::config::Cascade as _;
use plumb::rig::Site;

pub fn read() -> Result<Site, String> {
    let mut held =
        Site::default().merge(Site::env("PLUMB_SITE").map_err(|error| error.to_string())?);
    if held.turns == 0 {
        held.turns = Site::default().turns;
    }
    held.api = held.api.trim_end_matches('/').to_string();
    if !held.api.starts_with("https://") && !loopback(&held.api) {
        return Err("PLUMB_SITE_API must use https".into());
    }
    Ok(held)
}

fn loopback(api: &str) -> bool {
    ["http://127.0.0.1:", "http://localhost:", "http://[::1]:"]
        .iter()
        .any(|prefix| api.starts_with(prefix))
}

pub fn require(held: &Site) -> Result<(), String> {
    if held.token.contains(['\r', '\n']) {
        return Err("PLUMB_SITE_TOKEN contains a line break".into());
    }
    if held.domain.contains(['/', '\\']) {
        return Err("PLUMB_SITE_DOMAIN must be a hostname".into());
    }
    let missing = [
        ("PLUMB_SITE_ACCOUNT", held.account.as_str()),
        ("PLUMB_SITE_DOMAIN", held.domain.as_str()),
        ("PLUMB_SITE_TOKEN", held.token.as_str()),
    ]
    .into_iter()
    .filter_map(|(key, value)| value.trim().is_empty().then_some(key))
    .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("missing {}", missing.join(", ")))
    }
}
