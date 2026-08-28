use super::{Plan, seat};

#[test]
fn yaml() {
    let root = seat("yaml-project");
    root.wrote(
        "charts/probe/Chart.yaml",
        "apiVersion: v2\nname: probe\nversion: 1.0.0\nappVersion: \"1.0.0\"\ndescription: held\n",
    );
    root.git(&["commit", "-m", "source"]);
    root.wrote(
        "charts/probe/Chart.yaml",
        "apiVersion: v2\nname: probe\nversion: 2.0.0\nappVersion: \"2.0.0\"\ndescription: held\n",
    );
    let project = &[
        "ship/chart=charts/probe/Chart.yaml#/version",
        "ship/chart=charts/probe/Chart.yaml#/appVersion",
    ];
    let (held, ok) = root.planned(Plan {
        base: Some("HEAD"),
        world: &[],
        identity: &["marker=v2.0.0"],
        project,
        roots: &["ship/chart=charts/probe"],
        inventory: None,
    });
    assert!(ok, "{held}");
    let held: serde_json::Value = serde_json::from_str(&held).expect("held plan");
    let chart = held["actions"]
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["name"] == "ship/chart"))
        .expect("chart action");
    assert_eq!(chart["reason"], "input-held");
    assert_eq!(chart["run"], false);
    assert_eq!(
        chart["project"][0]["omit"].as_array().map(Vec::len),
        Some(2)
    );

    root.wrote(
        "charts/probe/Chart.yaml",
        "apiVersion: v2\nname: probe\nversion: 2.0.0\nappVersion: \"2.0.0\"\ndescription: moved\n",
    );
    let (moved, ok) = root.planned(Plan {
        base: Some("HEAD"),
        world: &[],
        identity: &["marker=v2.0.0"],
        project,
        roots: &["ship/chart=charts/probe"],
        inventory: None,
    });
    assert!(ok, "{moved}");
    let moved: serde_json::Value = serde_json::from_str(&moved).expect("moved plan");
    let chart = moved["actions"]
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["name"] == "ship/chart"))
        .expect("chart action");
    assert_eq!(chart["reason"], "input-moved");
    assert_eq!(chart["run"], true);
}
