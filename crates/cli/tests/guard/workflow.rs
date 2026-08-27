#[path = "workflow/fixture.rs"]
mod fixture;
#[path = "workflow/lane.rs"]
mod lane;
#[path = "workflow/plan.rs"]
mod plan;
#[path = "workflow/record.rs"]
mod record;
#[path = "workflow/reuse.rs"]
mod reuse;

use fixture::{Plan, digest, seat};

const PAIR: &str = "[workflow.hash.guard]\n\
\"rust\" = [\"crates\", \"Cargo.toml\"]\n\
\"web\" = [\"apps\"]\n";

#[test]
fn reads() {
    let root = seat("reads");
    root.declared(PAIR);
    let (text, ok) = root.shown();
    assert!(ok, "{text}");
    assert!(text.contains("guard/rust"), "{text}");
    assert!(text.contains("guard/web"), "{text}");
}

#[test]
fn expands() {
    let root = seat("expands");
    root.declared(&format!(
        "{PAIR}\"proofs\" = [\"key://guard/rust\", \"key://guard/web\"]\n"
    ));
    let (text, ok) = root.shown();
    assert!(ok, "{text}");
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with("guard/proofs"))
        .expect("proofs");
    assert!(line.contains("3 paths"), "{line}");
}

#[test]
fn refuses() {
    let root = seat("refuses");
    root.declared(&format!("{PAIR}\"proofs\" = [\"key://guard/gone\"]\n"));
    let (text, ok) = root.shown();
    assert!(!ok, "{text}");
    assert!(text.contains("names no key called guard/gone"), "{text}");
}

#[test]
fn cycles() {
    let root = seat("cycles");
    root.declared(
        "[workflow.hash.guard]\n\
         \"rust\" = [\"key://guard/web\"]\n\
         \"web\" = [\"key://guard/rust\"]\n",
    );
    let (text, ok) = root.shown();
    assert!(!ok, "{text}");
    assert!(text.contains("key cycle"), "{text}");
}

#[test]
fn lanes() {
    let root = seat("lanes");
    root.declared("[workflow.hash.nowhere]\n\"rust\" = [\"crates\"]\n");
    let (text, ok) = root.shown();
    assert!(!ok, "{text}");
    assert!(text.contains("names no lane called nowhere"), "{text}");
}

#[test]
fn suites() {
    let root = seat("suites");
    root.declared("[workflow.hash.guard]\n\"rust\" = [\"suite://cargo\"]\n");
    let (text, ok) = root.shown();
    assert!(ok, "{text}");
    assert!(text.contains("suite://cargo"), "{text}");
    assert!(text.contains("Cargo.toml"), "{text}");
}

#[test]
fn unknown() {
    let root = seat("unknown");
    root.declared("[workflow.hash.guard]\n\"rust\" = [\"suite://gone\"]\n");
    let (text, ok) = root.shown();
    assert!(!ok, "{text}");
    assert!(text.contains("names no suite called gone"), "{text}");
}

#[test]
fn namespaces() {
    let root = seat("namespaces");
    root.declared("[workflow.hash.ship.cargo]\n\"rehearse\" = [\"suite://cargo\"]\n");
    let (text, ok) = root.shown();
    assert!(ok, "{text}");
    assert!(text.contains("ship/cargo.rehearse"), "{text}");
}

#[test]
fn moves() {
    let root = seat("moves");
    root.declared(PAIR);
    let (first, _) = root.shown();
    root.wrote("apps/web/src/app.ts", "export const held = 1;\n");
    let (second, _) = root.shown();
    assert_eq!(digest(&first, "guard/rust"), digest(&second, "guard/rust"));
    assert_ne!(digest(&first, "guard/web"), digest(&second, "guard/web"));
}

#[test]
fn declarations() {
    let root = seat("declarations");
    root.declared(PAIR);
    let (first, _) = root.shown();
    root.declared(
        "[workflow.hash.guard]\n\
         \"rust\" = [\"crates\", \"Cargo.toml\", \"Cargo.lock\"]\n\
         \"web\" = [\"apps\"]\n",
    );
    let (second, _) = root.shown();
    assert_ne!(digest(&first, "guard/rust"), digest(&second, "guard/rust"));
}

#[test]
fn loose() {
    let root = seat("loose");
    root.declared(PAIR);
    let (text, ok) = root.verb("hash", "guard/rust", false);
    assert!(!ok, "{text}");
    assert!(text.contains("runs"), "{text}");
}

#[test]
fn forced() {
    let root = seat("forced");
    root.declared(PAIR);
    let (text, ok) = root.verb("hash", "guard/rust", true);
    assert!(!ok, "{text}");
    assert!(text.contains("forces every step"), "{text}");
}

#[test]
fn absent() {
    let root = seat("absent");
    root.declared(PAIR);
    let (text, ok) = root.verb("hash", "guard/gone", false);
    assert!(!ok, "{text}");
}

#[test]
fn tolerates() {
    let root = seat("tolerates");
    root.declared(PAIR);
    let (text, ok) = root.verb("lock", "guard/rust", false);
    assert!(ok, "{text}");
}

#[test]
fn seated() {
    let root = seat("seated");
    root.declared(PAIR);
    let (text, ok) = root.verb("lock", "guard/rust", false);
    assert!(ok, "{text}");
    let (text, ok) = root.verb("hash", "guard/rust", false);
    assert!(!ok, "{text}");
    assert!(text.contains("names no lock seat"), "{text}");
}
