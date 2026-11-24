use runseal::github::records::Author;

pub fn check(
    issue: u64,
    owner: Option<Author>,
    viewer: impl FnOnce() -> Result<Author, String>,
) -> Result<(), String> {
    let owner = owner
        .filter(qualified)
        .ok_or("Auto issue has no operation writer identity")?;
    let operator = viewer()?;
    if !qualified(&operator) {
        return Err("provider identity is unreadable".into());
    }
    if operator.id != owner.id {
        return Err(format!(
            "Auto #{issue} has operation writer {} (id {}), but this follow runs as {} (id {}); only the writer settles its automation-owned Auto, so nothing was written; rerun follow as {}",
            owner.login, owner.id, operator.login, operator.id, owner.login
        ));
    }
    Ok(())
}

fn qualified(actor: &Author) -> bool {
    actor.id > 0 && !actor.login.is_empty()
}
