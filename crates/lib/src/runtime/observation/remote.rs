use locus::{Candidate, Role};
use serde_json::json;
use std::time::{Duration, Instant};

pub struct Exchange<'a> {
    pub provider: &'static str,
    pub operation: &'a str,
    pub transport: &'static str,
    pub outcome: &'static str,
    pub status: Option<u16>,
    pub attempts: u8,
    pub elapsed: Duration,
}

pub fn record(exchange: &Exchange<'_>) {
    let Some((engine, context)) = super::view() else {
        return;
    };
    let elapsed = u64::try_from(exchange.elapsed.as_millis()).unwrap_or(u64::MAX);
    let candidate = Candidate::event(json!({
        "event": "provider.call",
        "provider": exchange.provider,
        "operation": exchange.operation,
        "transport": exchange.transport,
        "outcome": exchange.outcome,
        "status": exchange.status,
        "attempts": exchange.attempts,
        "duration_ms": elapsed,
    }))
    .ensure(Role::trace())
    .ensure(Role::span());
    let _ = engine.append(context, candidate);
}

#[cfg(any(feature = "vendor", feature = "windtun"))]
pub fn github(observation: &runseal::github::Observation) {
    let Some((engine, context)) = super::view() else {
        return;
    };
    let candidate = Candidate::event(json!({
        "event": "provider.call",
        "provider": observation.provider,
        "operation": observation.operation,
        "transport": observation.transport,
        "outcome": observation.outcome,
        "cause": observation.cause,
        "status": observation.status,
        "attempts": observation.attempts,
        "duration_ms": observation.elapsed,
        "partial": observation.partial,
        "wait": observation.wait,
    }))
    .ensure(Role::trace())
    .ensure(Role::span());
    let _ = engine.append(context, candidate);
}

pub fn git(args: &[&str], started: Instant, ok: bool) {
    let Some(operation) = operation(args) else {
        return;
    };
    let outcome = if ok { "ok" } else { "error" };
    record(&Exchange {
        provider: "git",
        operation,
        transport: "git",
        outcome,
        status: None,
        attempts: 1,
        elapsed: started.elapsed(),
    });
}

fn operation(args: &[&str]) -> Option<&'static str> {
    let mut index = 0;
    while index < args.len() {
        let arg = args[index];
        if matches!(arg, "-C" | "-c" | "--git-dir" | "--work-tree") {
            index += 2;
            continue;
        }
        if arg.starts_with('-') {
            index += 1;
            continue;
        }
        return match arg {
            "fetch" => Some("git.fetch"),
            "push" => Some("git.push"),
            "ls-remote" => Some("git.ls-remote"),
            "pull" => Some("git.pull"),
            "clone" => Some("git.clone"),
            _ => None,
        };
    }
    None
}

#[cfg(test)]
mod tests;
