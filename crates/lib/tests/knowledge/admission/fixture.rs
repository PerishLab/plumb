use base64::{Engine, engine::general_purpose::STANDARD};
use plumb::windtun::qualification::{Fault, Read};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use std::{cell::RefCell, collections::BTreeMap};

pub const WHARF: &str = "https://api.github.com/repos/PerishLab/wharf";
pub const COMMENT: &str = "https://api.github.com/repos/PerishLab/.github/issues/comments/7";
pub const LOCATION: &str = "https://github.com/PerishLab/.github/issues/178#issuecomment-7";
pub const PERMISSION: &str =
    "https://api.github.com/repos/PerishLab/plumb/collaborators/alice/permission";

pub struct Fixture {
    pub values: BTreeMap<String, Vec<u8>>,
    pub moved: bool,
    pub edited: bool,
    reads: RefCell<BTreeMap<String, usize>>,
}

impl Fixture {
    pub fn new() -> Self {
        let mut held = Self {
            values: BTreeMap::new(),
            moved: false,
            edited: false,
            reads: RefCell::new(BTreeMap::new()),
        };
        for (name, id, node) in [
            ("wharf", 1374358737_u64, "R_kgDOUesM0Q"),
            ("plumb", 10, "R_plumb"),
            (".github", 20, "R_governance"),
        ] {
            held.put(&format!("https://api.github.com/repos/PerishLab/{name}"), json!({"id":id,"node_id":node,"full_name":format!("PerishLab/{name}"),"default_branch":"main","owner":{"id":330236468_u64,"node_id":"O_kgDOE68CNA","login":"PerishLab"},"visibility":"public","archived":false,"disabled":false}));
        }
        let head = "a".repeat(40);
        held.put(
            &format!("{WHARF}/branches/main"),
            json!({"name":"main","protected":true,"commit":{"sha":head}}),
        );
        held.put(
            &format!("{WHARF}/git/commits/{head}"),
            json!({"sha":head,"tree":{"sha":"b".repeat(40)}}),
        );
        held.policy(b"{\"schema\":\"wharf.release.semantics.requirements/v1\",\"scopes\":[]}");
        let judgment = json!({"schema":"plumb.release.review/v1","product":"plumb","before":"a".repeat(64),"after":"b".repeat(64),"anchor":"c".repeat(64),"decision":"same","basis":"Reviewed the exact source/schema/assertion materials and conditions; no expressed boundary changed."});
        let body = format!(
            "Bound review.\n\n<!-- plumb.release.review/v1\n{}\n-->",
            serde_json::to_string(&judgment).unwrap()
        );
        held.put(COMMENT, json!({"id":7,"node_id":"IC_native","html_url":LOCATION,"issue_url":"https://api.github.com/repos/PerishLab/.github/issues/178","body":body,"user":{"id":23,"login":"alice","type":"User"}}));
        held.put(PERMISSION, json!({"permission":"write","role_name":"maintain","user":{"id":23,"login":"alice","type":"User"}}));
        held
    }

    pub fn put(&mut self, url: &str, value: Value) {
        self.values
            .insert(url.into(), serde_json::to_vec(&value).unwrap());
    }

    pub fn edit(&mut self, url: &str, change: impl FnOnce(&mut Value)) {
        let mut value = serde_json::from_slice(&self.values[url]).unwrap();
        change(&mut value);
        self.put(url, value);
    }

    pub fn policy(&mut self, raw: &[u8]) {
        let mut hash = Sha1::new();
        hash.update(format!("blob {}\0", raw.len()).as_bytes());
        hash.update(raw);
        self.put(&format!("{WHARF}/contents/resources/check/semantics.json?ref={}", "a".repeat(40)), json!({"type":"file","path":"resources/check/semantics.json","encoding":"base64","size":raw.len(),"sha":format!("{:x}",hash.finalize()),"content":STANDARD.encode(raw)}));
    }
}

impl Read for Fixture {
    fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>, Fault> {
        let mut reads = self.reads.borrow_mut();
        let count = reads.entry(url.into()).or_default();
        *count += 1;
        let raw = self
            .values
            .get(url)
            .ok_or_else(|| Fault::unknown("fixture has no native observation"))?;
        if *count > 1 {
            let moved = self.moved && url.ends_with("/branches/main");
            let edited = self.edited && url == COMMENT;
            if moved || edited {
                return Err(Fault::unknown("native observation moved"));
            }
        }
        if raw.len() > limit {
            return Err(Fault::refused("fixture read exceeds bounds"));
        }
        Ok(raw.clone())
    }

    fn image(&self, _: &str) -> Result<String, Fault> {
        Err(Fault::refused(
            "semantic readers must not inspect or execute images",
        ))
    }
}
