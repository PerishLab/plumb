use super::{Store, stop, truth};
use serde_json::{Value, json};
use std::fs;

impl Store {
    fn comments(&self, payload: &Value) -> Value {
        let mut comments = self.read("comments.json");
        if !truth(payload) {
            return json!([comments]);
        }
        let body = payload["body"].as_str().unwrap();
        if body.contains("plumb.auto-noop/v1") && self.0.join("commentfault").exists() {
            fs::remove_file(self.0.join("commentfault")).unwrap();
            stop("fixture interrupted before no-op receipt", 1);
        }
        let mut comment = payload.clone();
        comment["user"] = json!({"id":1,"login":"fixture-writer[bot]","type":"Bot"});
        comments.as_array_mut().unwrap().push(comment);
        self.write("comments.json", &comments);
        if body.contains("plumb.auto-absorbed/v1") && self.0.join("receiptfault").exists() {
            fs::remove_file(self.0.join("receiptfault")).unwrap();
            stop("fixture interrupted after absorption receipt", 1);
        }
        let mut issue = self.read("issue.json");
        issue["updated_at"] = format!(
            "2026-10-09T00:00:{:02}Z",
            comments.as_array().unwrap().len()
        )
        .into();
        self.write("issue.json", &issue);
        if body.contains("plumb.auto-noop/v1") && self.0.join("move").exists() {
            let target = fs::read_to_string(self.0.join("move")).unwrap();
            assert!(
                self.git(&["update-ref", "refs/heads/main", target.trim()])
                    .status
                    .success()
            );
            fs::remove_file(self.0.join("move")).unwrap();
        }
        payload.clone()
    }

    fn refs(&self) -> Value {
        let reference = self.reference();
        let result = self.git(&["show-ref", "--verify", "--hash", &reference]);
        if !result.status.success() {
            return json!([[]]);
        }
        let reference = json!({"ref":reference,"object":{"sha":String::from_utf8(result.stdout).unwrap().trim()}});
        json!([[reference]])
    }

    fn pull(&self, endpoint: &str, payload: &Value) -> Value {
        if !truth(payload) && endpoint.ends_with(&format!("/pulls/{}", self.number() + 1)) {
            return self.read("pull.json");
        }
        if !truth(payload) {
            return if self.0.join("pull.json").exists() {
                json!([[self.read("pull.json")]])
            } else {
                json!([[]])
            };
        }
        if !endpoint.ends_with("/pulls") {
            let mut pull = self.read("pull.json");
            pull.as_object_mut()
                .unwrap()
                .extend(payload.as_object().unwrap().clone());
            self.write("pull.json", &pull);
            return pull;
        }
        let head = json!({"ref":format!("auto/{}", self.number()),"sha":self.head(),"repo":{"full_name":"Example/probe"}});
        let pull = json!({"number":self.number() + 1,"state":"open","head":head,"base":{"ref":"main"},"body":payload["body"]});
        self.write("pull.json", &pull);
        if self.0.join("fault").exists() {
            fs::remove_file(self.0.join("fault")).unwrap();
            stop("fixture interrupted after provider write", 17);
        }
        pull
    }

    fn guard(&self) -> Value {
        if self.0.join("mergefault").exists() || self.0.join("passing").exists() {
            let run = json!({"name":"Guard","app":{"slug":"github-actions"},"head_sha":self.head(),"status":"completed","conclusion":"success","details_url":"https://github.com/Example/probe/actions/runs/17"});
            return json!([{"check_runs":[run]}]);
        }
        if self.0.join("guard.json").exists() {
            return json!([{"check_runs":[self.read("guard.json")]}]);
        }
        json!([{"check_runs":[]}])
    }

    pub(super) fn recovery(&self, endpoint: &str, payload: &Value) -> Value {
        if endpoint == "graphql" {
            let viewer = if self.0.join("viewer").exists() {
                self.read("viewer")
            } else {
                json!({"databaseId":1,"login":"fixture-writer"})
            };
            return json!({"data":{"viewer":viewer}});
        }
        if endpoint.contains("/issues?") {
            let issue = self.read("issue.json");
            return if self.0.join("discover").exists() && issue["state"] == "open" {
                json!([[issue]])
            } else {
                json!([[]])
            };
        }
        if endpoint.contains("/git/matching-refs/") {
            return self.refs();
        }
        if endpoint.contains("/git/ref/") {
            if self.0.join("pushfault").exists() {
                fs::remove_file(self.0.join("pushfault")).unwrap();
                stop("fixture interrupted after branch push", 1);
            }
            return json!({"object":{"sha":self.head()}});
        }
        if endpoint.contains("/pulls") {
            return self.pull(endpoint, payload);
        }
        if endpoint.contains("/check-runs") {
            return self.guard();
        }
        if endpoint.contains("/actions/runs/") {
            if self.0.join("run.json").exists() {
                return self.read("run.json");
            }
            return json!({"head_sha":self.head(),"path":".github/workflows/guard.yml","event":"pull_request","status":"completed","conclusion":"success"});
        }
        if endpoint.contains("/comments") {
            return self.comments(payload);
        }
        if endpoint.contains("/sub_issues") || endpoint.contains("/dependencies/") {
            return json!([[]]);
        }
        let mut issue = self.read("issue.json");
        if truth(payload) {
            issue
                .as_object_mut()
                .unwrap()
                .extend(payload.as_object().unwrap().clone());
            self.write("issue.json", &issue);
            if self.0.join("closefault").exists() {
                fs::remove_file(self.0.join("closefault")).unwrap();
                stop("fixture interrupted after closure write", 17);
            }
        }
        issue
    }
}
