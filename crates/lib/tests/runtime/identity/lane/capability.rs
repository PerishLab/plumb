use plumb::lane::{Action, Address, Capabilities, Name};
use serde_json::json;

struct Profile {
    name: &'static str,
    actions: Vec<Action>,
    allowed: Action,
    refused: Action,
}

#[test]
fn canonical() {
    let capabilities = Capabilities::new(vec![Action::Dispose, Action::Inspect, Action::Deploy]);
    let capabilities = capabilities.unwrap();
    let value = json!(["inspect", "deploy", "dispose"]);
    assert_eq!(serde_json::to_value(&capabilities).unwrap(), value);
    assert_eq!(
        serde_json::from_value::<Capabilities>(value).unwrap(),
        capabilities
    );
    assert!(capabilities.supports(Action::Deploy));
    assert!(capabilities.require(Action::Deploy).is_ok());
    assert!(!capabilities.supports(Action::Install));
    assert!(capabilities.require(Action::Install).is_err());
}

#[test]
fn refusals() {
    assert!(Capabilities::new(vec![Action::Publish, Action::Publish]).is_err());
    for value in [
        json!(["publish", "publish"]),
        json!(["Inspect"]),
        json!(["repair"]),
        json!(["fallback"]),
        json!(["merge"]),
        json!(["rollback"]),
        json!(["overwrite"]),
        json!(["retire"]),
        json!(null),
        json!({"deploy":true}),
    ] {
        assert!(serde_json::from_value::<Capabilities>(value).is_err());
    }
    let none = Capabilities::new(vec![]).unwrap();
    assert_eq!(serde_json::to_value(&none).unwrap(), json!([]));
    assert!(none.require(Action::Inspect).is_err());
}

#[test]
fn profiles() {
    let artifact = super::artifact::sample();
    assert!(artifact.build().is_none());
    for profile in [
        Profile {
            name: "preview",
            actions: vec![Action::Inspect, Action::Deploy, Action::Dispose],
            allowed: Action::Dispose,
            refused: Action::Build,
        },
        Profile {
            name: "lane.focus",
            actions: vec![Action::Inspect, Action::Install, Action::Uninstall],
            allowed: Action::Uninstall,
            refused: Action::Dispose,
        },
        Profile {
            name: "lane.focus",
            actions: vec![Action::Inspect, Action::Publish, Action::Install],
            allowed: Action::Publish,
            refused: Action::Dispose,
        },
    ] {
        let address = Address::new(
            "PerishLab/crest".into(),
            "crest".into(),
            Name::read(profile.name).unwrap(),
        );
        let address = address.unwrap();
        let capabilities = Capabilities::new(profile.actions).unwrap();
        assert!(capabilities.require(profile.allowed).is_ok());
        assert!(capabilities.require(profile.refused).is_err());
        let value = json!({"address":address,"artifact":artifact,"capabilities":capabilities});
        assert!(value["artifact"].get("url").is_none());
        assert!(value["artifact"].get("stamp").is_none());
        assert_eq!(value["artifact"]["build"], json!(null));
    }
}

#[test]
fn representations() {
    let lane = Name::read("lane.focus").unwrap();
    let artifact = super::artifact::sample();
    let binary = json!({"lane":lane,"representation":"lane.focus","artifact":artifact});
    let package = json!({"lane":lane,"representation":"v1.2.3-lane.focus","artifact":artifact});
    assert_eq!(binary["lane"], package["lane"]);
    assert_eq!(binary["artifact"], package["artifact"]);
    assert_ne!(binary["representation"], package["representation"]);
    assert!(Name::read(package["representation"].as_str().unwrap()).is_err());
    assert!(artifact.build().is_none());
    let package =
        Capabilities::new(vec![Action::Inspect, Action::Publish, Action::Install]).unwrap();
    assert!(package.require(Action::Dispose).is_err());
    assert!(package.require(Action::Uninstall).is_err());
}
