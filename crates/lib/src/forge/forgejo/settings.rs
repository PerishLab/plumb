use crate::config::Cascade as _;
use crate::rig::{Harness, Millis};
use std::collections::BTreeMap;

pub fn vars(url: String) -> Result<BTreeMap<String, String>, String> {
    let mut vars = BTreeMap::from([(
        "FORGEJO_URL".to_string(),
        std::env::var("FORGEJO_URL").unwrap_or(url),
    )]);
    for key in [
        "FORGEJO_TOKEN",
        "FORGEJO_TOKEN_FILE",
        "RUNSEAL_FORGEJO_ISSUER_NAMESPACE",
        "RUNSEAL_FORGEJO_ISSUER_WORKLOAD",
        "RUNSEAL_FORGEJO_ISSUER_CONTAINER",
        "RUNSEAL_FORGEJO_STORE_NAMESPACE",
        "RUNSEAL_FORGEJO_STORE_POD",
        "RUNSEAL_FORGEJO_STORE_DATABASE",
        "RUNSEAL_FORGEJO_STORE_USER",
    ] {
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
    held.run.poll = nonzero(held.run.poll, base.run.poll);
    held.run.timeout = nonzero(held.run.timeout, base.run.timeout);
    held.guard.register = nonzero(held.guard.register, base.guard.register);
    held.guard.pending = nonzero(held.guard.pending, base.guard.pending);
    held.guard.timeout = nonzero(held.guard.timeout, base.guard.timeout);
    Ok(held)
}

fn nonzero(value: Millis, fallback: Millis) -> Millis {
    if value == Millis::default() {
        fallback
    } else {
        value
    }
}
