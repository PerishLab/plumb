mod projection;

use super::release::{Compile, compile};
use super::world::{Fixture, SPEC};

fn measured(out: &std::path::Path) -> serde_json::Value {
    let text = std::fs::read_to_string(out.join("seal.json")).expect("seal");
    let seal: serde_json::Value = serde_json::from_str(&text).expect("seal json");
    seal["inputs"].clone()
}

#[test]
fn inputs() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    let tools = root.join("tools");
    let artifacts = root.join("artifacts");
    std::fs::create_dir_all(&tools).expect("tool root");
    std::fs::create_dir_all(&artifacts).expect("artifact root");
    std::fs::create_dir_all(root.join("charts/probe")).expect("chart root");
    let fixture = Fixture {
        root,
        tools: &tools,
    };
    fixture.seed();
    std::fs::write(
        root.join("plumb.toml"),
        format!("{SPEC}[release.chart]\nregistry = \"example.invalid\"\nchart = \"owner/probe\"\naccount = \"Example\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("charts/probe/Chart.yaml"), "name: probe\n").expect("chart");
    fixture.track("charts");
    let candidate = fixture.candidate();
    fixture.tag("v1.2.0-beta.7");
    fixture.archive(&artifacts, "v1.2.0-beta.7");

    let out = root.join("beta");
    compile(Compile {
        fixture: &fixture,
        artifacts: &artifacts,
        channel: "beta",
        version: "v1.2.0-beta.7",
        out: &out,
        promotion: None,
        commit: &candidate,
    });
    let seal: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("seal.json")).expect("seal"))
            .expect("seal json");
    let held = &seal["inputs"]["chart"];
    assert!(held["hash"].as_str().is_some_and(|hash| hash.len() == 64));
    assert_eq!(held["since"], "v1.2.0-beta.7");
    let bare = root.join("releases/v1");
    std::fs::create_dir_all(bare.join("channels")).expect("published root");
    std::fs::write(bare.join("bare.json"), "{}\n").expect("published seal");
    std::fs::write(
        bare.join("channels/stable.json"),
        format!(
            concat!(
                r#"{{"schema":1,"product":"probe","channel":"stable","releaseVersion":"v1.1.0","#,
                r#""commit":"{}","managers":{{}},"seal":{{"name":"seal.json","#,
                r#""mime":"application/json","sha256":"{}","size":3,"#,
                r#""url":"https://releases.test/v1/bare.json"}}}}"#
            ),
            candidate,
            "0".repeat(64)
        ),
    )
    .expect("stable pointer");
    compile(Compile {
        fixture: &fixture,
        artifacts: &artifacts,
        channel: "beta",
        version: "v1.2.0-beta.7",
        out: &root.join("bare"),
        promotion: None,
        commit: &candidate,
    });

    let published = root.join("releases/v1");
    std::fs::create_dir_all(published.join("channels")).expect("published root");
    std::fs::write(
        published.join("channels/stable.json"),
        format!(
            concat!(
                r#"{{"schema":1,"product":"probe","channel":"stable","releaseVersion":"v1.1.0","#,
                r#""commit":"{}","managers":{{}},"seal":{{"name":"seal.json","#,
                r#""mime":"application/json","sha256":"{}","size":3,"#,
                r#""url":"https://releases.test/v1/seal.json"}}}}"#
            ),
            candidate,
            "0".repeat(64)
        ),
    )
    .expect("stable pointer");
    std::fs::copy(out.join("seal.json"), published.join("seal.json")).expect("published seal");
    let next = root.join("held");
    compile(Compile {
        fixture: &fixture,
        artifacts: &artifacts,
        channel: "beta",
        version: "v1.2.0-beta.8",
        out: &next,
        promotion: None,
        commit: &candidate,
    });
    let seal: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(next.join("seal.json")).expect("seal"))
            .expect("seal json");
    assert_eq!(
        seal["inputs"]["chart"]["since"], "v1.2.0-beta.7",
        "an unchanged object still reports where it last changed"
    );
    assert_eq!(seal["releaseVersion"], "v1.2.0-beta.8");

    let recorded = measured(&out)["chart"]["hash"]
        .as_str()
        .expect("hash")
        .to_string();

    std::fs::write(
        root.join("charts/probe/Chart.yaml"),
        "name: probe\n# loose\n",
    )
    .expect("working tree edit");
    let dirty = root.join("dirty");
    compile(Compile {
        fixture: &fixture,
        artifacts: &artifacts,
        channel: "beta",
        version: "v1.2.0-beta.7",
        out: &dirty,
        promotion: None,
        commit: &candidate,
    });
    assert_eq!(
        measured(&dirty)["chart"]["hash"],
        recorded,
        "an unstaged edit is not part of the candidate"
    );
    fixture.track("charts");
    let staged = root.join("staged");
    compile(Compile {
        fixture: &fixture,
        artifacts: &artifacts,
        channel: "beta",
        version: "v1.2.0-beta.7",
        out: &staged,
        promotion: None,
        commit: &candidate,
    });
    assert_ne!(
        measured(&staged)["chart"]["hash"],
        recorded,
        "a recorded edit moves the object"
    );
}

#[test]
fn carried() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    let tools = root.join("tools");
    let artifacts = root.join("artifacts");
    std::fs::create_dir_all(&tools).expect("tool root");
    std::fs::create_dir_all(&artifacts).expect("artifact root");
    let fixture = Fixture {
        root,
        tools: &tools,
    };
    fixture.seed();
    std::fs::write(
        root.join("plumb.toml"),
        format!(
            "{SPEC}[release.oci]\nregistry = \"example.invalid\"\nimage = \"owner/probe\"\naccount = \"Example\"\n\n[release.chart]\nregistry = \"example.invalid\"\nchart = \"owner/probe\"\naccount = \"Example\"\n"
        ),
    )
    .expect("manifest");
    std::fs::write(root.join("Containerfile"), "FROM scratch\n").expect("containerfile");
    std::fs::create_dir_all(root.join("charts/probe")).expect("chart root");
    std::fs::write(root.join("charts/probe/Chart.yaml"), "name: probe\n").expect("chart");
    fixture.track("Containerfile");
    fixture.track("charts");
    let candidate = fixture.candidate();
    fixture.tag("v1.2.0-beta.7");
    fixture.archive(&artifacts, "v1.2.0-beta.7");

    let out = root.join("beta");
    compile(Compile {
        fixture: &fixture,
        artifacts: &artifacts,
        channel: "beta",
        version: "v1.2.0-beta.7",
        out: &out,
        promotion: None,
        commit: &candidate,
    });
    let held = measured(&out);
    assert!(
        held["oci"]["hash"].as_str().is_some_and(|h| h.len() == 64),
        "a declared image is an object the seal can speak about: {held}"
    );
    assert_eq!(held["oci"]["since"], "v1.2.0-beta.7");

    fixture.tag("v1.2.0-beta.8");
    fixture.archive(&artifacts, "v1.2.0-beta.8");
    let next = root.join("later");
    compile(Compile {
        fixture: &fixture,
        artifacts: &artifacts,
        channel: "beta",
        version: "v1.2.0-beta.8",
        out: &next,
        promotion: None,
        commit: &candidate,
    });
    let moved = measured(&next);
    assert_ne!(
        moved["oci"]["hash"], held["oci"]["hash"],
        "an image wrapping this release's archive moves with the release, whatever the Containerfile did"
    );
    assert_eq!(
        moved["chart"]["hash"], held["chart"]["hash"],
        "an object whose source did not move keeps its hash"
    );
}
