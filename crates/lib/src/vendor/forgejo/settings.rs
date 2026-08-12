use crate::config::Cascade as _;
use crate::rig::{Forgejo, Harness};

pub fn token() -> Result<String, String> {
    Ok(Forgejo::default()
        .merge(Forgejo::env("FORGEJO").map_err(|error| error.to_string())?)
        .token
        .trim()
        .to_string())
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
