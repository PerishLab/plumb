use super::{PAIR, Plan, seat};

#[test]
fn renders() {
    let root = seat("plans");
    root.declared(PAIR);
    root.git(&["commit", "-m", "base"]);
    root.wrote("apps/web/src/app.ts", "export const held = 1;\n");
    let (text, ok) = root.plan(
        Some("HEAD"),
        &["atom=PerishLab/actions@012345", "runner=forge@sha256:abc"],
    );
    assert!(ok, "{text}");
    let plan: serde_json::Value = serde_json::from_str(&text).expect("plan");
    assert_eq!(plan["schema"], "plumb.workflow-plan/v1");
    assert_eq!(plan["world"]["runner"], "forge@sha256:abc");
    assert_eq!(plan["actions"][0]["name"], "guard/rust");
    assert_eq!(plan["actions"][0]["run"], false);
    assert_eq!(plan["actions"][1]["name"], "guard/web");
    assert_eq!(plan["actions"][1]["run"], true);
    assert_eq!(plan["actions"][1]["reuse"]["type"], "none");
    root.wrote("irrelevant.txt", "outside every declared action\n");
    let (again, ok) = root.plan(
        Some("HEAD"),
        &["atom=PerishLab/actions@012345", "runner=forge@sha256:abc"],
    );
    assert!(ok, "{again}");
    let again: serde_json::Value = serde_json::from_str(&again).expect("plan");
    assert_eq!(again["identity"], plan["identity"]);
}

#[test]
fn ambiguity() {
    let root = seat("world");
    root.declared(PAIR);
    root.git(&["commit", "-m", "base"]);
    let (text, ok) = root.plan(Some("HEAD"), &["runner=one", "runner=two"]);
    assert!(!ok, "{text}");
}

#[test]
fn declaration() {
    let root = seat("declaration");
    root.declared(PAIR);
    root.git(&["commit", "-m", "base"]);
    root.declared(
        "[workflow.hash.guard]\n\
         \"rust\" = [\"crates\", \"Cargo.toml\", \"Cargo.lock\"]\n\
         \"web\" = [\"apps\"]\n",
    );
    let (text, ok) = root.plan(Some("HEAD"), &[]);
    assert!(ok, "{text}");
    let plan: serde_json::Value = serde_json::from_str(&text).expect("plan");
    assert_eq!(plan["actions"][0]["reason"], "input-moved");
    assert_eq!(plan["actions"][1]["reason"], "input-held");
}

#[test]
fn cold() {
    let root = seat("cold");
    root.declared(PAIR);
    root.git(&["commit", "-m", "source"]);
    let (text, ok) = root.plan(None, &[]);
    assert!(ok, "{text}");
    let plan: serde_json::Value = serde_json::from_str(&text).expect("plan");
    assert_eq!(plan["base"], serde_json::Value::Null);
    assert_eq!(plan["actions"][0]["reason"], "action-added");
    assert_eq!(plan["actions"][0]["decision"], "run");
    assert_eq!(plan["actions"][1]["reason"], "action-added");
    assert_eq!(plan["actions"][1]["decision"], "run");
}

#[test]
fn project() {
    let root = seat("project");
    root.declared(PAIR);
    root.wrote(
        "apps/web/package.json",
        r#"{"name":"@perish/web","version":"1.0.0","source":"index.js"}"#,
    );
    root.git(&["commit", "-m", "source"]);
    root.wrote(
        "apps/web/package.json",
        r#"{"name":"@perish/web","version":"2.0.0","source":"index.js"}"#,
    );
    let (held, ok) = root.planned(Plan {
        base: Some("HEAD"),
        world: &[],
        identity: &["version=2.0.0"],
        project: &["guard/web=apps/web/package.json#/version"],
        inventory: None,
    });
    assert!(ok, "{held}");
    let held: serde_json::Value = serde_json::from_str(&held).expect("held plan");
    assert_eq!(held["actions"][1]["run"], false);
    assert_eq!(held["actions"][1]["reason"], "input-held");
    assert_eq!(
        held["actions"][1]["project"][0]["path"],
        "apps/web/package.json"
    );
    assert_eq!(held["actions"][1]["project"][0]["omit"][0], "/version");

    root.wrote(
        "apps/web/package.json",
        r#"{"name":"@perish/web","version":"2.0.0","source":"next.js"}"#,
    );
    let (moved, ok) = root.planned(Plan {
        base: Some("HEAD"),
        world: &[],
        identity: &["version=2.0.0"],
        project: &["guard/web=apps/web/package.json#/version"],
        inventory: None,
    });
    assert!(ok, "{moved}");
    let moved: serde_json::Value = serde_json::from_str(&moved).expect("moved plan");
    assert_eq!(moved["actions"][1]["run"], true);

    let (text, ok) = root.planned(Plan {
        base: Some("HEAD"),
        world: &[],
        identity: &[],
        project: &["guard/web=apps/web/package.json#/gone"],
        inventory: None,
    });
    assert!(!ok, "{text}");
}

#[test]
fn infers() {
    let root = seat("infers");
    root.wrote("pnpm-lock.yaml", "lockfileVersion: 9\n");
    root.git(&["commit", "-m", "base"]);
    root.wrote("apps/web/src/app.ts", "export const held = 2;\n");
    let (text, ok) = root.plan(Some("HEAD"), &[]);
    assert!(ok, "{text}");
    let plan: serde_json::Value = serde_json::from_str(&text).expect("plan");
    let actions = plan["actions"].as_array().expect("actions");
    assert_eq!(actions.len(), 3);
    assert_eq!(actions[0]["name"], "guard/rust");
    assert_eq!(actions[0]["run"], false);
    assert_eq!(actions[1]["name"], "guard/test");
    assert_eq!(actions[1]["run"], false);
    assert_eq!(actions[2]["name"], "guard/web");
    assert_eq!(actions[2]["run"], true);
}
