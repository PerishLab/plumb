use plumb::land::rejoin::{Report, report};

pub(in crate::command) fn rejoin() -> Result<String, String> {
    let root = super::worktree::root()?;
    let report = report(&root).map_err(|refusal| refusal.message)?;
    Ok(said(&report))
}

fn said(report: &Report) -> String {
    let marker = report.marker.as_deref().unwrap_or_default();
    let commit = report.commit.as_deref().unwrap_or_default();
    let lacking = report.lacking.join(", ");
    match (report.state, report.conflicted) {
        ("unmarked", _) => "origin holds no stable marker; nothing to rejoin".to_string(),
        ("home", _) => {
            format!("stable {marker} at {commit} is home: main holds every change it carries")
        }
        (_, true) => format!(
            "stable {marker} at {commit} conflicts with main in {lacking}; land the release line's changes into main first"
        ),
        _ => format!(
            "stable {marker} at {commit} carries changes main lacks in {lacking}; land the release line's changes into main first"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::said;
    use plumb::land::rejoin::{Report, SCHEMA};

    fn report(state: &'static str, lacking: &[&str], conflicted: bool) -> Report {
        Report {
            schema: SCHEMA,
            root: ".".into(),
            marker: Some("v1.0.0".into()),
            commit: Some("c1".into()),
            main: Some("m1".into()),
            state,
            lacking: lacking.iter().map(|held| held.to_string()).collect(),
            conflicted,
        }
    }

    #[test]
    fn reads() {
        assert_eq!(
            said(&report("home", &[], false)),
            "stable v1.0.0 at c1 is home: main holds every change it carries"
        );
        let owed = said(&report("owed", &["fix.txt", "lib.rs"], false));
        assert!(owed.contains("lacks in fix.txt, lib.rs"), "{owed}");
        assert!(
            owed.contains("land the release line's changes into main"),
            "{owed}"
        );
        let conflicted = said(&report("owed", &["fix.txt"], true));
        assert!(
            conflicted.contains("conflicts with main in fix.txt"),
            "{conflicted}"
        );
    }
}
