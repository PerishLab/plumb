use super::{binary::Binary, material::Codec};
use plumb::windtun::qualification::{Fault, Read, Result};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap};

pub const ROOT: &str = "https://releases.keel.perish.uk/v1/releases/rc/v1.0.0-rc.1/windtun";
pub const GH: &str = "https://api.github.com/repos/PerishLab/wharf";

pub struct Change {
    pub path: String,
    pub after: usize,
    pub body: Vec<u8>,
}

pub struct Fixture {
    pub bodies: BTreeMap<String, Vec<u8>>,
    pub counts: RefCell<BTreeMap<String, usize>>,
    pub change: Option<Change>,
    pub drift: bool,
    pub pass: Value,
    pub result: Value,
    pub declaration: Value,
    pub pins: Value,
}

impl Fixture {
    pub fn new() -> Self {
        let mut fixture = Self {
            bodies: BTreeMap::new(),
            counts: RefCell::new(BTreeMap::new()),
            change: None,
            drift: false,
            pass: Value::Null,
            result: Value::Null,
            declaration: Value::Null,
            pins: json!({}),
        };
        fixture.source();
        fixture.product();
        fixture.runner();
        fixture.scene();
        fixture.receipt();
        fixture
    }

    pub fn set(&mut self, url: &str, value: &Value) {
        self.bodies.insert(url.into(), Codec::bytes(value));
    }
    pub fn get(&self, url: &str) -> Value {
        serde_json::from_slice(&self.bodies[url]).unwrap()
    }

    pub fn policy(&mut self, implementation: &str) {
        let policy = json!({"schema":"wharf.windtun.requirements/v1", "policy":"wharf.windtun.linux/v1", "implementation":implementation});
        self.set(
            &format!(
                "{GH}/contents/resources/check/windtun.json?ref={}",
                "a".repeat(40)
            ),
            &Codec::content("resources/check/windtun.json", &Codec::bytes(&policy)),
        );
    }

    fn source(&mut self) {
        for (product, id) in [("wharf", 1374358737_u64), ("keel", 17)] {
            let repository = format!("PerishLab/{product}");
            self.set(&format!("https://api.github.com/repos/{repository}"), &json!({"id":id,"node_id":if product=="wharf" {"R_kgDOUesM0Q"} else {"keel"},"full_name":repository,"default_branch":"main","visibility":"public","archived":false,"disabled":false,"owner":{"id":330236468_u64,"node_id":"O_kgDOE68CNA","login":"PerishLab"}}));
        }
        self.set(
            &format!("{GH}/branches/main"),
            &json!({"name":"main","protected":true,"commit":{"sha":"a".repeat(40)}}),
        );
        self.set(
            &format!("{GH}/git/commits/{}", "a".repeat(40)),
            &json!({"sha":"a".repeat(40),"tree":{"sha":"b".repeat(40)}}),
        );
        self.policy(&"c".repeat(64));
        let source = Binary::source("keel", "v1.0.0-rc.1");
        let root = "https://api.github.com/repos/PerishLab/keel";
        self.set(
            &format!("{root}/git/ref/tags/v1.0.0-rc.1"),
            &json!({"ref":"refs/tags/v1.0.0-rc.1","object":{"type":"tag","sha":"5".repeat(40)}}),
        );
        for (tag, commit) in [("5", "1"), ("6", "0")] {
            self.set(&format!("{root}/git/tags/{}",tag.repeat(40)),&json!({"sha":tag.repeat(40),"tag":"v1.0.0-rc.1","object":{"type":"commit","sha":commit.repeat(40)}}));
            self.set(
                &format!("{root}/git/commits/{}", commit.repeat(40)),
                &json!({"sha":commit.repeat(40),"tree":{"sha":source["tree"]}}),
            );
        }
        let text = "[identity]\nns=\"keel\"\n[release]\nauthority=\"https://releases.keel.perish.uk\"\nbinaries=[\"keel\"]\ntargets=[\"x86_64-unknown-linux-gnu\"]\n[windtun]\ncases=[\"health\"]\nimage=\"ghcr.io/perishlab/keel\"\n[windtun.scenes]\nauthority=\"https://depot.seatrial.perish.uk\"\nproduct=\"seatrial\"\nchannel=\"beta\"\n";
        self.set(
            &format!(
                "https://api.github.com/repos/PerishLab/keel/contents/plumb.toml?ref={}",
                "1".repeat(40)
            ),
            &Codec::content("plumb.toml", text.as_bytes()),
        );
        self.declaration = json!({"schema":"wharf.trial.declaration/v1","source":source,"authority":"https://releases.keel.perish.uk","manifest":Codec::sha(text.as_bytes()),"specification":plumb::windtun::parse(text).unwrap().unwrap()});
    }

    pub fn publication(&mut self, source: &Value, channel: &str) -> Value {
        let product = source["repository"]
            .as_str()
            .unwrap()
            .split('/')
            .next_back()
            .unwrap();
        let authority = format!("https://releases.{product}.perish.uk");
        let raw = Binary::archive(source);
        let url = format!(
            "{authority}/v1/objects/sha256/{}/{product}.tar.gz",
            Codec::sha(&raw)
        );
        let archive = Codec::reference(&url, &raw);
        self.bodies.insert(url, raw);
        let url = format!(
            "{authority}/v1/releases/{channel}/{}/seal.json",
            source["marker"].as_str().unwrap()
        );
        let seal = json!({"schema":1,"product":product,"releaseVersion":source["marker"],"channel":channel,"commit":source["commit"],"generator":{},"provenance":{},"managers":{},"artifacts":{"linux-x64":archive},"url":url});
        let reference = Codec::reference(&url, &Codec::bytes(&seal));
        self.set(&url, &seal);
        let pointer = json!({"schema":1,"product":product,"releaseVersion":source["marker"],"channel":channel,"commit":source["commit"],"seal":reference,"managers":{}});
        self.set(&format!("{authority}/v1/channels/{channel}.json"), &pointer);
        json!({"pointer":pointer,"seal":seal})
    }

    fn product(&mut self) {
        let source = Binary::source("keel", "v1.0.0-rc.1");
        self.pins["product"] = self.publication(&source, "rc");
        let publisher = json!({"run":"9","attempt":"1","wharf":"e".repeat(40)});
        let distribution = json!({"schema":1,"repository":source["repository"],"marker":source["marker"],"commit":source["commit"],"tree":source["tree"],"state":"complete","attempt":publisher,"completed":publisher,"media":{"binaries":"published","oci":"published","cargo":"none","npm":"none","chart":"none","cfworker":"none","channel":"pointed"},"resolution":{},"trial":self.declaration});
        self.set(
            "https://releases.keel.perish.uk/v1/releases/rc/v1.0.0-rc.1/distribution.json",
            &distribution,
        );
    }

    fn runner(&mut self) {
        let source = Binary::source("seatrial", "v1.0.0");
        self.pins["runner"] = self.publication(&source, "stable");
        self.set(
            &format!(
                "https://api.github.com/repos/PerishLab/seatrial/git/commits/{}",
                "3".repeat(40)
            ),
            &json!({"sha":source["commit"],"tree":{"sha":source["tree"]}}),
        );
    }

    fn scene(&mut self) {
        use plumb::depot::v3::{Identity, Kind, Manifest, Marker, Object, Pointer, Publication};
        let raw = b"name = 'health'\n";
        let object = Object {
            path: "keel/health.toml".into(),
            sha256: Codec::sha(raw),
            size: raw.len() as u64,
            media: "application/toml".into(),
            executable: false,
        };
        let manifest = Manifest::new(
            Identity {
                product: "seatrial".into(),
                channel: "beta".into(),
                version: "v1.0.0-beta.1".into(),
                marker: Marker {
                    name: "v1.0.0-beta.1".into(),
                    sha256: "f".repeat(64),
                },
                kind: Kind::Scene,
            },
            vec![object],
        )
        .unwrap();
        let pointer = Pointer::new(
            &manifest,
            Publication {
                source: "https://depot.seatrial.perish.uk",
                prior: None,
                created: "2026-10-01T00:00:00Z".into(),
            },
        )
        .unwrap();
        let url = pointer.manifest.url.clone();
        self.bodies.insert(
            format!(
                "{}objects/keel/health.toml",
                url.strip_suffix("manifest.json").unwrap()
            ),
            raw.to_vec(),
        );
        self.bodies.insert(url, manifest.encode().unwrap());
        self.pins["scene"] = serde_json::to_value(&pointer).unwrap();
        self.set(
            "https://depot.seatrial.perish.uk/channels/beta/scenes/latest.json",
            &serde_json::to_value(pointer).unwrap(),
        );
    }
}

impl Read for &Fixture {
    fn bytes(&self, url: &str, limit: usize) -> Result<Vec<u8>> {
        let mut counts = self.counts.borrow_mut();
        let count = counts.entry(url.into()).or_default();
        *count += 1;
        let raw = match &self.change {
            Some(change) if change.path == url && *count >= change.after => &change.body,
            _ => self
                .bodies
                .get(url)
                .ok_or_else(|| Fault::unknown("fixture material absent"))?,
        };
        if raw.len() > limit {
            return Err(Fault::refused("fixture material exceeds bounds"));
        }
        Ok(raw.clone())
    }
    fn image(&self, reference: &str) -> Result<String> {
        if reference != "ghcr.io/perishlab/keel:1.0.0-rc.1" {
            return Err(Fault::unknown("unrecognized runtime"));
        }
        let mut counts = self.counts.borrow_mut();
        let count = counts.entry("image".into()).or_default();
        *count += 1;
        Ok(format!(
            "sha256:{}",
            if self.drift && *count > 1 { "d" } else { "b" }.repeat(64)
        ))
    }
}
