use plumb::land::rejoin::{Report, plan, run};

pub(in crate::command) fn rejoin(dry: bool) -> Result<String, String> {
    let root = super::worktree::root()?;
    let report = if dry { plan(&root) } else { run(&root) }.map_err(|refusal| refusal.message)?;
    Ok(said(&report))
}

fn said(report: &Report) -> String {
    let marker = report.marker.as_deref().unwrap_or_default();
    let commit = report.commit.as_deref().unwrap_or_default();
    match report.state {
        "unmarked" => "origin holds no stable marker; nothing to rejoin".to_string(),
        "home" => format!("stable {marker} at {commit} is already an ancestor of main"),
        "owed" => report.steps.join("\n"),
        _ => format!(
            "rejoined {marker} at {commit} into main through {}",
            report.url.as_deref().unwrap_or_default()
        ),
    }
}
