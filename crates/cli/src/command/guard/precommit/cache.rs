use plumb::guard::Action;
use std::path::PathBuf;

fn seat(proof: &Action) -> Result<PathBuf, String> {
    let home = plumb::config::value("PLUMB_HOME")
        .map(PathBuf::from)
        .or_else(|| plumb::config::data("plumb"))
        .ok_or_else(|| "cannot cache guard action: no PLUMB_HOME".to_string())?;
    Ok(home
        .join("proof")
        .join("guard")
        .join("actions")
        .join(format!("{}.json", proof.world)))
}

pub(super) fn contains(proof: &Action) -> bool {
    seat(proof)
        .ok()
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<Action>(&bytes).ok())
        .as_ref()
        == Some(proof)
}

pub(super) fn record(proof: &Action) -> Result<(), String> {
    let path = seat(proof)?;
    let parent = path
        .parent()
        .ok_or_else(|| "guard action has no cache parent".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(proof).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("cannot write {}: {error}", path.display()))
}
