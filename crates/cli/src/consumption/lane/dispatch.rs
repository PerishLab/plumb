use super::{Execution, hub, plan};
use plumb::lane::dispatch::Operation;

pub(super) fn execute(
    options: Execution,
    operation: Operation,
) -> Result<serde_json::Value, String> {
    let held = plan::reviewed(&options.reviewed)?;
    if held.dispatch.operation() != operation {
        return Err("explicit lane command differs from the reviewed plan operation".into());
    }
    held.observe(&options.target.root)?;
    hub::observe(&held)?;
    hub::submit(&held)?;
    Ok(serde_json::json!({
        "schema":"plumb.lane.dispatch/v1", "plan":held.digest()?, "dispatch":held.dispatch,
        "submission":"acknowledged", "operation":{"outcome":"unknown","reason":"unverified"},
        "current":{"outcome":"unknown","reason":"unverified"}
    }))
}
