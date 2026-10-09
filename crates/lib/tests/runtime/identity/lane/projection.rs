use plumb::lane::operation::{Authorization, Intent, Material, State};
use serde_json::json;

#[test]
fn empty() {
    let unknown = json!({"state":"unknown","credentials":"private"});
    assert!(serde_json::from_value::<Authorization>(unknown.clone()).is_err());
    assert!(serde_json::from_value::<State>(unknown).is_err());
    let absent = json!({"state":"absent","payload":"private"});
    assert!(serde_json::from_value::<State>(absent.clone()).is_err());
    assert!(serde_json::from_value::<Material>(absent).is_err());
    for action in ["inspect", "uninstall", "dispose"] {
        let value = json!({"action":action,"payload":"private"});
        assert!(serde_json::from_value::<Intent>(value).is_err());
    }
}
