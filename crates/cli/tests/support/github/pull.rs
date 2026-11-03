use super::{Store, stop};
use serde_json::{Value, json};
use std::{fs, process::Command};

impl Store {
    pub(super) fn ready(&self) -> Value {
        let states = if self.0.join("ready.json").exists() {
            self.read("ready.json")
        } else {
            json!(["CLEAN"])
        };
        let states = states.as_array().unwrap();
        if states.len() > 1 {
            self.write("ready.json", &json!(&states[1..]));
        }
        json!({"headRefOid":self.head(),"mergeStateStatus":states[0]})
    }

    fn actor(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("--git-dir")
            .arg(self.0.join("origin.git"))
            .args(args)
            .env("GIT_AUTHOR_NAME", "GitHub")
            .env("GIT_AUTHOR_EMAIL", "noreply@github.com")
            .env("GIT_COMMITTER_NAME", "GitHub")
            .env("GIT_COMMITTER_EMAIL", "noreply@github.com")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().into()
    }

    pub(super) fn merge(&self, args: &[String]) -> Value {
        if self.0.join("policyfault").exists() {
            fs::remove_file(self.0.join("policyfault")).unwrap();
            stop(
                "X Pull request Example/probe#18 is not mergeable: the base branch policy prohibits the merge.",
                1,
            );
        }
        let argument = |flag: &str| &args[args.iter().position(|arg| arg == flag).unwrap() + 1];
        let candidate = argument("--match-head-commit");
        assert_eq!(candidate, &self.head());
        let tree = format!("{candidate}^{{tree}}");
        let parent = format!("{candidate}^");
        let message = format!("{}\n\n{}", argument("--subject"), argument("--body"));
        let merge = self.actor(&["commit-tree", &tree, "-p", &parent, "-m", &message]);
        self.actor(&["update-ref", "refs/heads/main", &merge, &parent]);
        self.actor(&[
            "update-ref",
            &format!("refs/pull/{}/head", self.number() + 1),
            candidate,
        ]);
        let mut pull = self.read("pull.json");
        pull.as_object_mut().unwrap().extend(json!({"state":"closed","merged":true,"merged_at":"2026-10-09T00:00:00Z","merge_commit_sha":merge}).as_object().unwrap().clone());
        self.write("pull.json", &pull);
        json!({})
    }
}
