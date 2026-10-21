use super::{Operation, Parts};
use crate::lane::operation::State;

pub(super) fn check(parts: &Parts) -> Result<(), String> {
    if parts.schema != "wharf.lane.dispatch/v1"
        || !parts.target.repository().starts_with("PerishLab/")
    {
        return Err("unsupported static lane dispatch identity".into());
    }
    if parts.revision == 0 || parts.revision == u64::MAX {
        return Err("unsupported static lane dispatch revision".into());
    }
    match &parts.expected {
        State::Absent {} => {}
        State::Present { revision, .. } if *revision > 0 && *revision < parts.revision => {}
        _ => return Err("lane dispatch requires an exact settled precondition".into()),
    }
    match (parts.operation, &parts.source) {
        (Operation::Apply, Some(source)) => source.validate()?,
        (Operation::Inspect | Operation::Dispose, None) => {}
        _ => return Err("apply needs source; inspect/dispose must not carry source".into()),
    }
    if parts.operation == Operation::Dispose && !matches!(parts.expected, State::Present { .. }) {
        return Err("dispose requires an explicitly present artifact".into());
    }
    Login::read(&parts.caller)?;
    let prefix = "PerishLab/wharf/.github/workflows/lane.yml@";
    let commit = parts
        .workflow
        .strip_prefix(prefix)
        .ok_or("lane needs the fixed Wharf workflow")?;
    super::super::digest::hex(commit, 40)
}

struct Login;

impl Login {
    fn read(value: &str) -> Result<(), String> {
        if value.is_empty() || value.len() > 39 {
            return Err("lane caller needs a bounded GitHub login".into());
        }
        if value.starts_with('-') || value.ends_with('-') || value.contains("--") {
            return Err("lane caller has invalid login segments".into());
        }
        if !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return Err("lane caller has invalid login characters".into());
        }
        Ok(())
    }
}
