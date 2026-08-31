use super::{Plan, seat};

#[test]
fn rooted() {
    let root = seat("rooted-action");
    root.git(&["add", "-A"]);
    root.git(&["commit", "-m", "source"]);
    let (image, ok) = root.planned(Plan {
        base: Some("HEAD"),
        world: &[],
        identity: &["marker=v2.0.0"],
        project: &[],
        roots: &["ship/oci=*"],
        inventory: None,
    });
    assert!(ok, "{image}");
    let image: serde_json::Value = serde_json::from_str(&image).expect("image plan");
    let image = image["actions"]
        .as_array()
        .and_then(|actions| actions.iter().find(|action| action["name"] == "ship/oci"))
        .expect("root-only image action");
    assert_eq!(image["run"], false);
    assert_eq!(image["project"], serde_json::json!([]));
}
