use super::super::{Fault, Result, wire::Wire};
use serde_json::Value;
use std::cmp::Ordering;

const SCHEMA: &str = "wharf.plan.reference/v1";

const LEGACY: &[&str] = &[
    "schema",
    "repository",
    "marker",
    "commit",
    "tree",
    "media",
    "state",
    "attempt",
    "completed",
    "resolution",
    "trial",
];

const CARRIED: &[&str] = &[
    "schema",
    "repository",
    "marker",
    "commit",
    "tree",
    "media",
    "state",
    "attempt",
    "completed",
    "resolution",
    "trial",
    "plan",
];

const NESTED: &[&str] = &[
    "schema",
    "repository",
    "marker",
    "commit",
    "tree",
    "wharf",
    "run",
    "attempt",
    "sha256",
];

pub(super) fn admit(record: &Value) -> Result<()> {
    shape(record)?;
    match record.get("plan") {
        None => Ok(()),
        Some(plan) if plan.is_null() => Err(Fault::refused("null plan reference")),
        Some(_) => reference(record),
    }
}

fn shape(record: &Value) -> Result<()> {
    let carried = record
        .as_object()
        .ok_or_else(|| Fault::refused("expected an object"))?
        .contains_key("plan");
    Wire::fields(record, if carried { CARRIED } else { LEGACY })
}

fn reference(record: &Value) -> Result<()> {
    let plan = &record["plan"];
    Wire::fields(plan, NESTED)?;
    Wire::require(plan["schema"] == SCHEMA, "plan reference schema differs")?;
    for field in ["commit", "tree", "wharf"] {
        Wire::hex(&plan[field], 40)?;
    }
    Wire::hex(&plan["sha256"], 64)?;
    for field in ["repository", "marker"] {
        Wire::text(&plan[field])?;
    }
    decimal(&plan["run"])?;
    let attempt = decimal(&plan["attempt"])?;
    Wire::observer(&record["completed"])?;
    for field in ["repository", "marker", "commit", "tree"] {
        Wire::require(
            plan[field] == record[field],
            "plan reference source differs",
        )?;
    }
    Wire::require(
        plan["run"] == record["completed"]["run"],
        "plan reference run differs",
    )?;
    Wire::require(
        plan["wharf"] == record["completed"]["wharf"],
        "plan reference wharf differs",
    )?;
    Wire::require(
        within(attempt, Wire::text(&record["completed"]["attempt"])?),
        "plan reference attempt follows completion",
    )?;
    Ok(())
}

fn decimal(value: &Value) -> Result<&str> {
    let text = Wire::text(value)?;
    Wire::require(
        text.len() <= 20
            && !text.starts_with('0')
            && text.bytes().all(|byte| byte.is_ascii_digit()),
        "invalid plan identity",
    )?;
    Ok(text)
}

fn within(left: &str, right: &str) -> bool {
    match left.len().cmp(&right.len()) {
        Ordering::Less => true,
        Ordering::Equal => left <= right,
        Ordering::Greater => false,
    }
}
