use serde::Serialize;
use std::path::{Path, PathBuf};

use super::tree;

const CODE: &str = "guard.integration-branch";

#[derive(Serialize)]
pub struct Finding {
    schema: &'static str,
    root: PathBuf,
    ok: bool,
    code: &'static str,
    message: String,
    remedy: String,
}

impl Finding {
    pub fn render(&self, json: bool) {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(self).expect("Guard finding should encode")
            );
        } else {
            eprintln!("plumb guard: {}: {}", self.code, self.message);
            eprintln!("see: {}", self.remedy);
        }
    }
}

pub fn inspect(root: &Path) -> Result<(), Finding> {
    let branch = tree::git(
        root,
        &["branch", "--show-current"],
        "read the current branch",
    )
    .unwrap_or_default();
    match branch.as_str() {
        "main" | "master" => Err(Finding {
            schema: "plumb.guard-finding/v1",
            root: root.to_path_buf(),
            ok: false,
            code: CODE,
            message: format!(
                "{branch} is an integration branch; begin mutable work from an Issue-managed topic worktree"
            ),
            remedy: format!("plumb cookbook {CODE}"),
        }),
        _ => Ok(()),
    }
}
