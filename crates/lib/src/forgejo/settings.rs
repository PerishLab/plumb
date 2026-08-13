use crate::config::Cascade as _;
use crate::rig::Harness;
use std::collections::BTreeMap;

pub fn vars(url: String) -> Result<BTreeMap<String, String>, String> {
    let mut vars = BTreeMap::from([(
        "FORGEJO_URL".to_string(),
        std::env::var("FORGEJO_URL").unwrap_or(url),
    )]);
    for key in ["FORGEJO_TOKEN", "FORGEJO_TOKEN_FILE"] {
        if let Ok(value) = std::env::var(key)
            && !value.trim().is_empty()
        {
            vars.insert(key.to_string(), value);
        }
    }
    if !vars.contains_key("FORGEJO_TOKEN") && !vars.contains_key("FORGEJO_TOKEN_FILE") {
        return Err("FORGEJO_TOKEN or FORGEJO_TOKEN_FILE is required".into());
    }
    Ok(vars)
}

pub fn harness() -> Result<Harness, String> {
    let mut held = Harness::env("HARNESS")
        .map(|seen| Harness::default().merge(seen))
        .map_err(|error| error.to_string())?;
    let base = Harness::default();
    held.run_poll_ms = nonzero(held.run_poll_ms, base.run_poll_ms);
    held.run_timeout_ms = nonzero(held.run_timeout_ms, base.run_timeout_ms);
    held.guard_register_ms = nonzero(held.guard_register_ms, base.guard_register_ms);
    held.guard_pending_ms = nonzero(held.guard_pending_ms, base.guard_pending_ms);
    held.guard_timeout_ms = nonzero(held.guard_timeout_ms, base.guard_timeout_ms);
    Ok(held)
}

fn nonzero(value: u64, fallback: u64) -> u64 {
    if value == 0 { fallback } else { value }
}
