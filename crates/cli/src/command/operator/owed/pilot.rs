use super::{
    Seat,
    native::Native,
    successor::{Request, Stage},
};
use crate::shape::release::Spec;
use base64::{Engine, engine::general_purpose::STANDARD};
use plumb::windtun::qualification::{Fault, Read};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::process::Command;
use tempfile::TempDir;

pub(super) const OLD: &str = "v1.2.3";
pub(super) const NEXT: &str = "v1.2.4";
pub(super) const RC: &str = "v1.2.4-rc.1";
pub(super) const SOURCE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(super) const BASE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub(super) const TREE: &str = "cccccccccccccccccccccccccccccccccccccccc";
const TAG: &str = "dddddddddddddddddddddddddddddddddddddddd";
const HEAD: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const NAME: &str = "Example/seat";
const HOST: &str = "https://releases.seat.example";
pub(super) const DEBT: &str = "release.marker-distributed";

pub(super) struct Pilot {
    root: TempDir,
    next: String,
    rc: String,
    pub records: BTreeMap<String, Value>,
    pub listing: String,
    pub record: Value,
    pub blob: String,
    pub owed: Vec<&'static str>,
    pub revoked: bool,
    reads: std::cell::Cell<u32>,
}

pub(super) fn native(route: &str) -> String {
    format!("https://api.github.com/{route}")
}
pub(super) fn product(route: &str) -> String {
    native(&format!("repos/{NAME}/{route}"))
}
pub(super) fn receipt(marker: &str) -> String {
    let channel = if marker.contains("-rc.") {
        "rc"
    } else {
        "stable"
    };
    format!("{HOST}/v1/releases/{channel}/{marker}/distribution.json")
}
pub(super) fn channel() -> String {
    format!("{HOST}/v1/channels/stable.json")
}
fn digest(value: &Value) -> String {
    format!("{:x}", Sha256::digest(value.to_string()))
}
fn file(path: &str, raw: &[u8]) -> Value {
    let mut hash = Sha1::new();
    hash.update(format!("blob {}\0", raw.len()));
    hash.update(raw);
    json!({"path":path,"type":"file","encoding":"base64","content":STANDARD.encode(raw),"size":raw.len(),"sha":format!("{:x}",hash.finalize())})
}
fn reference(name: &str, kind: &str, sha: &str) -> Value {
    json!({"ref":format!("refs/{name}"),"object":{"type":kind,"sha":sha}})
}

impl Pilot {
    pub fn new() -> Self {
        Self::with(OLD, "v1.2.2", NEXT)
    }

    pub fn with(old: &str, prior: &str, next: &str) -> Self {
        let rc = format!("{next}-rc.1");
        let root = tempfile::tempdir().expect("temporary Git source");
        let mut this = Self {
            root,
            next: next.into(),
            rc: rc.clone(),
            records: BTreeMap::new(),
            listing: format!("{TAG}\trefs/tags/{old}\n{BASE}\trefs/tags/{old}^{{}}"),
            record: Value::Null,
            blob: String::new(),
            owed: vec![DEBT],
            revoked: false,
            reads: std::cell::Cell::new(0),
        };
        this.git(&["init", "-q"]);
        this.git(&["remote", "add", "origin", "git@github.com:Example/seat.git"]);
        let manifest=b"[identity]\nns='seat'\n[release]\nauthority='https://releases.seat.example'\nbinaries=['seat']\ntargets=['x86_64-unknown-linux-gnu']\n";
        std::fs::write(this.root.path().join("plumb.toml"), manifest).expect("manifest");
        let stale = json!({"schema":1,"repository":NAME,"marker":old,"commit":BASE,"tree":TREE,"state":"incomplete","media":{"binaries":"published","oci":"failed","channel":"unreached"}});
        let pointer = json!({"schema":1,"channel":"stable","product":"seat","releaseVersion":prior,"commit":BASE});
        this.record = json!({"schema":"plumb.release.successor/v1","repository":NAME,"repository_id":42,"authority":HOST,"predecessor":{"marker":old,"commit":BASE,"tree":TREE,"tag":TAG,"distribution_sha256":digest(&stale),"channel_sha256":digest(&pointer)},"successor":{"marker":next,"rc":rc,"commit":SOURCE,"tree":TREE}});
        this.records.insert(receipt(old), stale);
        this.records.insert(channel(), pointer);
        this.records.insert(
            native("repos/Example/seat"),
            json!({"full_name":NAME,"id":42}),
        );
        this.records.insert(
            native("repos/PerishLab/.github"),
            json!({"full_name":"PerishLab/.github","id":1388431417_u64,"node_id":"R_kgDOUsHIOQ","default_branch":"main"}),
        );
        this.records.insert(
            native("repos/PerishLab/.github/branches/main"),
            json!({"name":"main","protected":true,"commit":{"sha":HEAD}}),
        );
        this.records
            .insert(product("git/matching-refs/heads/release/"), json!([]));
        this.records.insert(
            product("tags?per_page=100&page=1"),
            json!([{"name":old,"commit":{"sha":BASE}}]),
        );
        this.records.insert(
            product("git/ref/heads/main"),
            reference("heads/main", "commit", SOURCE),
        );
        for commit in [BASE, SOURCE] {
            this.records.insert(
                product(&format!("git/commits/{commit}")),
                json!({"sha":commit,"tree":{"sha":TREE}}),
            );
            this.records.insert(
                product(&format!("contents/plumb.toml?ref={commit}")),
                file("plumb.toml", manifest),
            );
        }
        this.records.insert(
            product(&format!("compare/{BASE}...{SOURCE}")),
            json!({"merge_base_commit":{"sha":BASE},"status":"ahead"}),
        );
        this.tag(old, TAG, BASE, old);
        this.approve();
        this
    }

    pub fn git(&self, args: &[&str]) {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(self.root.path())
                .status()
                .expect("git")
                .success()
        );
    }

    pub fn approve(&mut self) {
        let path = format!("governance/release-successors/42/{}.json", self.next);
        let entry = file(&path, self.record.to_string().as_bytes());
        self.blob = entry["sha"].as_str().unwrap().into();
        self.records.insert(
            native(&format!(
                "repos/PerishLab/.github/contents/{path}?ref={HEAD}"
            )),
            entry,
        );
    }

    pub fn tag(&mut self, name: &str, sha: &str, target: &str, message: &str) {
        self.records.insert(
            product(&format!("git/ref/tags/{name}")),
            reference(&format!("tags/{name}"), "tag", sha),
        );
        self.records.insert(
            product(&format!("git/tags/{sha}")),
            json!({"tag":name,"sha":sha,"object":{"type":"commit","sha":target},"message":message}),
        );
    }

    pub fn line(&mut self) {
        self.records.insert(
            product("git/matching-refs/heads/release/"),
            json!([reference(
                &format!("heads/release/{}", self.next),
                "commit",
                SOURCE
            )]),
        );
    }

    pub fn marked(&mut self, marker: &str) {
        let sha = if marker == self.rc {
            "1111111111111111111111111111111111111111"
        } else {
            "2222222222222222222222222222222222222222"
        };
        let message = if marker == self.rc {
            self.rc.clone()
        } else {
            format!("{}\n\npromotes {}", self.next, self.rc)
        };
        self.tag(marker, sha, SOURCE, &message);
        self.listing.push_str(&format!(
            "\n{sha}\trefs/tags/{marker}\n{SOURCE}\trefs/tags/{marker}^{{}}"
        ));
        self.records
            .get_mut(&product("tags?per_page=100&page=1"))
            .unwrap()
            .as_array_mut()
            .unwrap()
            .push(json!({"name":marker,"commit":{"sha":SOURCE}}));
    }

    pub fn shipped(&mut self) {
        self.records.insert(receipt(&self.rc),json!({"schema":1,"repository":NAME,"marker":self.rc,"commit":SOURCE,"tree":TREE,"state":"complete"}));
    }

    pub fn inspect(&self, marker: &str, stage: Stage<'_>) -> Result<(), String> {
        let root = self.root.path();
        let spec = Spec::read(&root.join("plumb.toml"))?;
        let seat = Seat {
            root,
            listing: &self.listing,
            remote: "origin",
            spec: &spec,
        };
        seat.inspect(
            &Native(self),
            Request {
                blob: &self.blob,
                marker,
                stage,
            },
            |_| Ok(self.owed.clone()),
        )
        .map(|_| ())
    }
}

impl Read for Pilot {
    fn bytes(&self, url: &str, _: usize) -> Result<Vec<u8>, Fault> {
        if self.revoked
            && url == native("repos/PerishLab/.github/branches/main")
            && self.reads.replace(self.reads.get() + 1) > 0
        {
            let branch = json!({"name":"main","protected":true,"commit":{"sha":BASE}});
            return Ok(branch.to_string().into_bytes());
        }
        self.records
            .get(url)
            .map(|v| v.to_string().into_bytes())
            .ok_or_else(|| Fault::refused(format!("unavailable {url}")))
    }
    fn optional(&self, url: &str, _: usize) -> Result<Option<Vec<u8>>, Fault> {
        Ok(self.records.get(url).map(|v| v.to_string().into_bytes()))
    }

    fn image(&self, _: &str) -> Result<String, Fault> {
        Err(Fault::refused("unexpected image read"))
    }
}
