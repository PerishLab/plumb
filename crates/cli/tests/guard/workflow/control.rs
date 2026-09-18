use super::seat;

#[test]
fn separated() {
    let root = seat("control-plane");
    root.declared(
        "[workflow.hash.guard]\n\
         policy = [\"*\"]\n\
         [workflow.hash.ship]\n\
         binary = [\"*\"]\n",
    );
    root.git(&["commit", "-m", "base"]);
    root.wrote(".forgejo/workflows/ship.yml", "name: ship\n");
    let (text, ok) = root.plan(Some("HEAD"), &[]);
    assert!(ok, "{text}");
    let plan: serde_json::Value = serde_json::from_str(&text).expect("plan");
    let action = |name| {
        plan["actions"]
            .as_array()
            .and_then(|actions| actions.iter().find(|action| action["name"] == name))
            .expect("action")
    };
    assert_eq!(action("guard/policy")["reason"], "input-moved");
    assert_eq!(action("ship/binary")["reason"], "input-held");
}
