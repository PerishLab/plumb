use serde_json::{Value, json};

pub(super) struct World {
    pub root: tempfile::TempDir,
}

impl World {
    pub fn new() -> Self {
        let world = Self {
            root: super::preview::preview(),
        };
        world.write("package.json", &Self::package().to_string());
        world.write("wrangler.jsonc", &Self::worker().to_string());
        world
    }

    pub fn package() -> Value {
        json!({"name":"@perish/review","private":true,"scripts":{"build":"node build.mjs"}})
    }

    pub fn worker() -> Value {
        json!({"name":"crest-review","account_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "compatibility_date":"2026-10-08","workers_dev":false,
            "assets":{"directory":"./dist","not_found_handling":"404-page"},"previews":{}})
    }

    pub fn write(&self, name: &str, text: &str) {
        super::write(self.root.path(), &format!("reviews/crest/{name}"), text);
    }

    pub fn alias(&self, name: &str, text: &str) {
        let path = self.root.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        super::write(self.root.path(), name, text);
        super::preview::git(self.root.path(), &["add", name]);
    }

    pub fn judge(&self) -> String {
        let home = super::super::support::home();
        let output = super::super::support::plumb()
            .args(["doctor", "--json"])
            .arg(self.root.path())
            .env("PLUMB_HOME", home.path())
            .output()
            .unwrap();
        let held: Value = serde_json::from_slice(&output.stdout).unwrap();
        held["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|finding| finding["code"] == "structure.preview-static")
            .map(|finding| finding["evidence"].as_str().unwrap())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn check(&self, refused: bool) {
        let held = self.judge();
        assert_eq!(!held.is_empty(), refused, "{held}");
    }
}
