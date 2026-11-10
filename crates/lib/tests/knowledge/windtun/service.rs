use super::{material::Codec, world::Fixture};
use plumb::windtun::qualification::{Inspector, State};
use serde_json::{Value, json};

const DECLARATION: &str = "[windtun.services.locus]\nauthority=\"https://releases.locus.perish.uk\"\nchannel=\"stable\"\n[windtun.services.locus.placement]\nartifact=\"linux-x64-deb\"\nentry=\"locus-api\"\n[windtun.services.locus.placement.paths]\nlocus-api=\"usr/bin/locus-api\"\n";

pub fn fixture() -> Fixture {
    let mut f = Fixture::new();
    let url = format!(
        "https://api.github.com/repos/PerishLab/keel/contents/plumb.toml?ref={}",
        "1".repeat(40)
    );
    use base64::{Engine, engine::general_purpose::STANDARD};
    let prior = STANDARD
        .decode(f.get(&url)["content"].as_str().unwrap())
        .unwrap();
    let text = String::from_utf8(prior).unwrap() + DECLARATION;
    f.set(&url, &Codec::content("plumb.toml", text.as_bytes()));
    f.declaration["schema"] = json!("wharf.trial.declaration/v2");
    f.declaration["manifest"] = json!(Codec::sha(text.as_bytes()));
    f.declaration["specification"] =
        serde_json::to_value(plumb::windtun::parse(&text).unwrap().unwrap()).unwrap();
    let url = "https://releases.keel.perish.uk/v1/releases/rc/v1.0.0-rc.1/distribution.json";
    let mut distribution = f.get(url);
    distribution["trial"] = f.declaration.clone();
    f.set(url, &distribution);
    f.receipt();
    f.pass["schema"] = json!("wharf.windtun.pass/v2");
    let service = service(&mut f);
    f.pass["services"] = json!({"locus":service});
    let p = &service["pointer"];
    f.pass["products"].as_array_mut().unwrap().push(json!({"name":"locus","channel":"stable","version":p["releaseVersion"],"commit":p["commit"],"seal":p["seal"]["sha256"],"archive":service["seal"]["artifacts"]["linux-x64-deb"]["sha256"],"placement":service["declaration"]["placement"]}));
    f.publish();
    f
}

fn service(f: &mut Fixture) -> Value {
    let root = "https://api.github.com/repos/PerishLab/locus";
    let repository = json!({"id":23,"node":"locus","name":"PerishLab/locus","default":"main"});
    f.set(root, &json!({"id":23,"node_id":"locus","full_name":"PerishLab/locus","default_branch":"main","visibility":"public","archived":false,"disabled":false,"owner":{"id":330236468_u64,"node_id":"O_kgDOE68CNA","login":"PerishLab"}}));
    let source = json!({"repository":"PerishLab/locus","marker":"v0.16.0","commit":"7".repeat(40),"tree":"8".repeat(40)});
    let tag = "9".repeat(40);
    f.set(
        &format!("{root}/git/ref/tags/v0.16.0"),
        &json!({"ref":"refs/tags/v0.16.0","object":{"type":"tag","sha":tag}}),
    );
    f.set(
        &format!("{root}/git/tags/{tag}"),
        &json!({"sha":tag,"tag":"v0.16.0","object":{"type":"commit","sha":source["commit"]}}),
    );
    f.set(
        &format!("{root}/git/commits/{}", "7".repeat(40)),
        &json!({"sha":source["commit"],"tree":{"sha":source["tree"]}}),
    );
    let location = "https://releases.locus.perish.uk/v1/objects/service.deb";
    let raw = b"opaque verified service fixture";
    f.bodies.insert(location.into(), raw.to_vec());
    let archive = Codec::reference(location, raw);
    let url = "https://releases.locus.perish.uk/v1/releases/stable/v0.16.0/seal.json";
    let seal = json!({"schema":1,"product":"locus","releaseVersion":"v0.16.0","channel":"stable","commit":source["commit"],"generator":{},"provenance":{},"managers":{},"artifacts":{"linux-x64-deb":archive},"url":url});
    let pointer = json!({"schema":1,"product":"locus","releaseVersion":"v0.16.0","channel":"stable","commit":source["commit"],"seal":Codec::reference(url,&Codec::bytes(&seal)),"managers":{}});
    f.set(url, &seal);
    f.set(
        "https://releases.locus.perish.uk/v1/channels/stable.json",
        &pointer,
    );
    let publisher = json!({"run":"11","attempt":"1","wharf":"e".repeat(40)});
    let distribution = json!({"schema":1,"repository":source["repository"],"marker":source["marker"],"commit":source["commit"],"tree":source["tree"],"state":"complete","attempt":publisher,"completed":publisher,"media":{"binaries":"published","cargo":"none","oci":"none","npm":"none","chart":"none","cfworker":"none","channel":"pointed"},"resolution":{},"trial":null});
    f.set(
        "https://releases.locus.perish.uk/v1/releases/stable/v0.16.0/distribution.json",
        &distribution,
    );
    json!({"declaration":f.declaration["specification"]["services"]["locus"],"pointer":pointer,"seal":seal,"distribution":distribution,"source":source,"tag":tag,"repository":repository})
}

#[test]
fn combination() {
    let f = fixture();
    let report = Inspector::new(&f).inspect("PerishLab/keel", "v1.0.0-rc.1");
    assert_eq!(report.state, State::Qualified, "{report:?}");
    assert!(report.evidence.unwrap()["world"]["services"]["locus"].is_object());
}

#[test]
fn declaration() {
    let f = fixture();
    let base = "[identity]\nns=\"keel\"\n[windtun]\ncases=[\"health\"]\nimage=\"ghcr.io/perishlab/images:stable\"\n[windtun.scenes]\nauthority=\"https://depot.seatrial.perish.uk\"\nproduct=\"seatrial\"\nchannel=\"stable\"\n";
    let text = base.to_string() + DECLARATION;
    for (from, to) in [
        ("services.locus", "services.keel"),
        (
            "channel=\"stable\"\n[windtun.services",
            "channel=\"v0.16.0\"\n[windtun.services",
        ),
        ("releases.locus.perish.uk", "evil.invalid"),
        ("linux-x64-deb", "linux-x64"),
        ("usr/bin/locus-api", "../bin/locus-api"),
        ("usr/bin/locus-api", "usr//bin/locus-api"),
        ("entry=", "unknown="),
    ] {
        assert!(
            plumb::windtun::parse(&text.replace(from, to)).is_err(),
            "{from}"
        );
    }
    let service: plumb::windtun::Service =
        serde_json::from_value(f.declaration["specification"]["services"]["locus"].clone())
            .unwrap();
    assert!(service.validate("locus").is_ok());
    assert!(service.validate("keel").is_err());
}

#[test]
fn identities() {
    for change in 0..8 {
        let mut f = fixture();
        match change {
            0 => {
                f.pass["products"].as_array_mut().unwrap().pop();
            }
            1 => {
                let value = f.pass["products"][1].clone();
                f.pass["products"].as_array_mut().unwrap().push(value);
            }
            2 => f.pass["products"][1]["name"] = json!("foreign"),
            3 => f.pass["products"][1]["placement"]["entry"] = json!("foreign"),
            4 => f.pass["products"][1]["archive"] = json!("0".repeat(64)),
            5 => f.pass["services"]["locus"]["pointer"]["commit"] = json!("0".repeat(40)),
            6 => f.pass["schema"] = json!("wharf.windtun.pass/v1"),
            _ => f.pass["services"]["extra"] = json!({}),
        }
        f.publish();
        let report = Inspector::new(&f).inspect("PerishLab/keel", "v1.0.0-rc.1");
        assert_eq!(report.state, State::Refused, "{change}: {report:?}");
    }
}

#[test]
fn movement() {
    let mut f = fixture();
    let url = "https://releases.locus.perish.uk/v1/channels/stable.json";
    let mut pointer = f.get(url);
    pointer["commit"] = json!("0".repeat(40));
    f.change = Some(super::world::Change {
        path: url.into(),
        after: 2,
        body: Codec::bytes(&pointer),
    });
    let report = Inspector::new(&f).inspect("PerishLab/keel", "v1.0.0-rc.1");
    assert_eq!(report.state, State::Refused, "{report:?}");
    assert!(!report.qualified);
}

#[test]
fn integrity() {
    let mut f = fixture();
    f.bodies
        .get_mut("https://releases.locus.perish.uk/v1/objects/service.deb")
        .unwrap()[0] ^= 1;
    let report = Inspector::new(&f).inspect("PerishLab/keel", "v1.0.0-rc.1");
    assert_eq!(report.state, State::Refused, "{report:?}");
}

mod baseline {
    use plumb::windtun::parse;

    const DECLARED: &str = r#"[guard.seatrial]
cases = ["health-lite", "again-lite"]
image = "ghcr.io/perishlab/keel"
[guard.seatrial.scenes]
authority = "https://depot.seatrial.perish.uk"
product = "seatrial"
channel = "beta"
"#;

    #[test]
    fn protection() {
        assert_eq!(parse("[guard.lock]\ntarget=\"host\"\n").unwrap(), None);
        assert!(parse("[guard]\nlock=true\n").is_err());
        assert!(parse("[guard]\nunknown=true\n").is_err());
        assert!(
            parse(&(DECLARED.to_owned() + "[guard.lock]\ntarget=\"host\"\n"))
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn products() {
        let keel = parse(DECLARED).unwrap().unwrap();
        assert_eq!(keel.cases, ["health-lite", "again-lite"]);
        let plumb = DECLARED.replace("[\"health-lite\", \"again-lite\"]", "[\"writer\"]");
        let plumb = plumb.replace("ghcr.io/perishlab/keel", "ghcr.io/perishlab/images:stable");
        assert!(parse(&plumb).unwrap().is_some());
        assert_eq!(parse("[identity]\nns = \"keel\"\n").unwrap(), None);
        let modern = DECLARED.replace("guard.seatrial", "windtun");
        assert_eq!(parse(&modern).unwrap(), parse(DECLARED).unwrap());
        assert!(parse(&(DECLARED.to_owned() + &modern)).is_err());
    }

    #[test]
    fn refuse() {
        for (from, to) in [
            ("channel = \"beta\"", "channel = \"v1.2.3-beta.1\""),
            (
                "https://depot.seatrial.perish.uk",
                "http://depot.seatrial.perish.uk",
            ),
            ("[\"health-lite\", \"again-lite\"]", "[]"),
            ("again-lite", "health-lite"),
            ("health-lite", "*"),
            ("ghcr.io/perishlab/keel", "ghcr.io/perishlab/keel:0.27.0"),
            ("image =", "command ="),
        ] {
            assert!(
                parse(&DECLARED.replace(from, to)).is_err(),
                "{from} -> {to}"
            );
        }
        assert!(parse(&(DECLARED.to_owned() + "version = \"v1.2.3\"\n")).is_err());
        let modern = DECLARED.replace("guard.seatrial", "windtun");
        assert!(parse(&modern.replace("again-lite", "health-lite")).is_err());
        assert!(parse(&modern.replace("channel = \"beta\"", "channel = \"v1.2.3\"")).is_err());
        assert!(parse(&modern.replace("image =", "command =")).is_err());
    }
}
