use super::{Store, truth};
use serde_json::{Value, json};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::{Command, Output};

impl Store {
    pub(super) fn read(&self, name: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(name)).unwrap()).unwrap()
    }

    pub(super) fn write(&self, name: &str, value: &Value) {
        fs::write(self.0.join(name), serde_json::to_vec(value).unwrap()).unwrap();
    }

    pub(super) fn git(&self, args: &[&str]) -> Output {
        Command::new("git")
            .arg("--git-dir")
            .arg(self.0.join("origin.git"))
            .args(args)
            .output()
            .unwrap()
    }

    pub(super) fn head(&self) -> String {
        let output = self.git(&["rev-parse", &self.reference()]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().into()
    }

    pub(super) fn number(&self) -> u64 {
        self.read("issue.json")["number"].as_u64().unwrap()
    }

    pub(super) fn reference(&self) -> String {
        format!("refs/heads/auto/{}", self.number())
    }

    pub(super) fn record(&self, args: &[String], payload: &Value) {
        let endpoint = &args[1];
        let call = json!({"endpoint":endpoint,"payload":payload,"args":args});
        let mut trace = OpenOptions::new()
            .append(true)
            .create(true)
            .open(self.0.join("calls"))
            .unwrap();
        let mut record = serde_json::to_vec(&call).unwrap();
        record.push(b'\n');
        trace.write_all(&record).unwrap();
    }

    pub(super) fn basic(&self, endpoint: &str, payload: &Value) -> Value {
        if !endpoint.contains("/comments") {
            return self
                .read("routes.json")
                .as_object()
                .unwrap()
                .get(endpoint)
                .unwrap()
                .clone();
        }
        let mut comments = self.read("comments.json");
        if !truth(payload) {
            return json!([comments]);
        }
        let mut comment = payload.clone();
        comment["user"] = json!({"id":1});
        comments.as_array_mut().unwrap().push(comment);
        self.write("comments.json", &comments);
        payload.clone()
    }
}
