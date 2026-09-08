use super::{PAIR, Plan, seat};

#[test]
fn unread() {
    let root = seat("unread-inventory");
    root.declared(PAIR);
    root.git(&["commit", "-m", "source"]);
    let store = crate::support::Bucket::open(4);
    let inventory = format!("{}/workflow/inventory.json", store.endpoint());
    let resolve = || {
        root.remote(
            Plan {
                base: None,
                world: &[],
                identity: &[],
                project: &[],
                roots: &[],
                inventory: None,
            },
            Some(&inventory),
        )
    };
    let (absent, ok) = resolve();
    assert!(ok, "{absent}");
    store.finish();
    let (unread, ok) = resolve();
    assert!(ok, "{unread}");
    let plan: serde_json::Value = serde_json::from_str(&unread).unwrap();
    assert!(
        plan["actions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["run"] == true)
    );
}

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
