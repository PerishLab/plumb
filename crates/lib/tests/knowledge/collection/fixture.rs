use plumb::seat::depot::v3::{self, Identity, Manifest, Object, Pointer, Publication};
use plumb::semantics::collection::{Collector, Report};
use plumb::semantics::subjects::Subjects;
use plumb::windtun::qualification::{Fault, Read};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use std::{cell::RefCell, collections::BTreeMap};

pub const BASE: &str = "https://api.github.com/repos/PerishLab/plumb";
pub const DEPOT: &str = "https://depot.seatrial.perish.uk";
pub const POINTER: &str = "https://depot.seatrial.perish.uk/channels/stable/scenes/latest.json";

pub struct Fixture {
    pub values: BTreeMap<String, Vec<u8>>,
    pub moved: bool,
    reads: RefCell<BTreeMap<String, usize>>,
}

impl Fixture {
    pub fn new() -> Self {
        let mut held = Self {
            values: BTreeMap::new(),
            moved: false,
            reads: RefCell::new(BTreeMap::new()),
        };
        held.source();
        held.scene(Some(Self::subjects()));
        held
    }

    pub fn subjects() -> Subjects {
        let cases: Vec<_> = ["writer", "hidden", "diagnostic"]
            .iter()
            .map(|id| {
                json!({
                    "id":id, "role":if *id == "diagnostic" {"diagnostic"} else {"normative"},
                    "subjects":["plumb"], "fixtures":["runseal"], "preparation":["ectropy"],
                    "conditions":[{"platform":"linux-x86_64","backend":"none"}],
                    "evidence":["group/main.py"]
                })
            })
            .collect();
        Subjects::parse(
            &serde_json::to_vec(
                &json!({"schema":Subjects::SCHEMA,"product":"seatrial","cases":cases}),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn put(&mut self, url: &str, value: Value) {
        self.values
            .insert(url.into(), serde_json::to_vec(&value).unwrap());
    }

    fn source(&mut self) {
        self.put(BASE, json!({"id":10,"node_id":"R_actual","full_name":"PerishLab/plumb", "owner":{"login":"PerishLab","id":330236468_u64,"node_id":"O_kgDOE68CNA"},"visibility":"public","archived":false,"disabled":false}));
        self.put(
            &format!("{BASE}/git/commits/{}", "a".repeat(40)),
            json!({"sha":"a".repeat(40), "tree":{"sha":"b".repeat(40)}}),
        );
        let text = "[identity]\nns='plumb'\n[windtun]\ncases=['writer']\nimage='ghcr.io/perishlab/plumb'\n[windtun.scenes]\nauthority='https://depot.seatrial.perish.uk'\nproduct='seatrial'\nchannel='stable'\n";
        let mut hash = Sha1::new();
        hash.update(format!("blob {}\0", text.len()).as_bytes());
        hash.update(text.as_bytes());
        let sha = format!("{:x}", hash.finalize());
        self.put(&format!("{BASE}/git/trees/{}?recursive=1", "b".repeat(40)),json!({"sha":"b".repeat(40),"truncated":false,"tree":[{"path":"plumb.toml","mode":"100644","type":"blob","sha":sha,"size":text.len()},{"path":"src","mode":"040000","type":"tree","sha":"e".repeat(40)},{"path":"src/lib.rs","mode":"100644","type":"blob","sha":"c".repeat(40),"size":13}]}));
        use base64::{Engine, engine::general_purpose::STANDARD};
        self.put(&format!("{BASE}/git/blobs/{sha}"), json!({"sha":sha,"size":text.len(),"encoding":"base64","content":STANDARD.encode(text)}));
    }

    pub fn scene(&mut self, subjects: Option<Subjects>) {
        let mut files = BTreeMap::new();
        let mut index = "format=2\n[[groups]]\nid='group'\nroot='group'\nsetup=[]\nbinding='generation'\nprofile='host/v1'\n".to_string();
        for id in ["writer", "hidden", "diagnostic"] {
            index.push_str(&format!(
                "[[cases]]\nid='{id}'\nmanifest='group/{id}.toml'\ngroup='group'\n"
            ));
            files.insert(
                format!("group/{id}.toml"),
                format!("default={}\n", id != "hidden").into_bytes(),
            );
        }
        files.insert("bundle.toml".into(), index.into_bytes());
        files.insert(
            "group/main.py".into(),
            b"assert promised_behavior\n".to_vec(),
        );
        if let Some(subjects) = subjects {
            files.insert(
                Subjects::PATH.into(),
                serde_json::to_vec(&subjects).unwrap(),
            );
        }
        let objects = files
            .iter()
            .map(|(path, raw)| Object {
                path: path.clone(),
                sha256: plumb::depot::sha(raw),
                size: raw.len() as u64,
                media: "text/plain".into(),
                executable: path.ends_with(".py"),
            })
            .collect();
        let manifest = Manifest::new(
            Identity {
                product: "seatrial".into(),
                channel: "stable".into(),
                version: "v0.2.0".into(),
                marker: v3::Marker {
                    name: "v0.2.0".into(),
                    sha256: "d".repeat(64),
                },
                kind: v3::Kind::Scene,
            },
            objects,
        )
        .unwrap();
        let pointer = Pointer::new(
            &manifest,
            Publication {
                source: DEPOT,
                prior: None,
                created: "2026-11-06T12:00:00Z".into(),
            },
        )
        .unwrap();
        self.values
            .insert(POINTER.into(), pointer.encode().unwrap());
        let standing = format!("{DEPOT}/channels/stable/scenes/versions/v0.2.0/latest.json");
        self.values.insert(standing, pointer.encode().unwrap());
        self.values
            .insert(pointer.manifest.url.clone(), manifest.encode().unwrap());
        let base = pointer.manifest.url.strip_suffix("manifest.json").unwrap();
        for (path, bytes) in files {
            self.values.insert(format!("{base}objects/{path}"), bytes);
        }
    }

    pub fn edit(&mut self, url: &str, change: impl FnOnce(&mut Value)) {
        let mut value: Value = serde_json::from_slice(&self.values[url]).unwrap();
        change(&mut value);
        self.put(url, value);
    }

    pub fn inspect(self) -> Report {
        Collector::new(self).inspect("PerishLab/plumb", &"a".repeat(40))
    }
}

impl Read for Fixture {
    fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>, Fault> {
        let mut reads = self.reads.borrow_mut();
        let count = reads.entry(url.into()).or_default();
        *count += 1;
        if self.moved && url == POINTER && *count > 1 {
            return Err(Fault::unknown("discovery moved"));
        }
        let raw = self
            .values
            .get(url)
            .ok_or_else(|| Fault::unknown("fixture has no observation"))?;
        if raw.len() > limit {
            return Err(Fault::refused("fixture exceeds read budget"));
        }
        Ok(raw.clone())
    }

    fn image(&self, _: &str) -> Result<String, Fault> {
        Err(Fault::refused("collection must not inspect images"))
    }
}
