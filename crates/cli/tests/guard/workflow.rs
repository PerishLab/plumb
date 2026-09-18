#[path = "workflow/fixture.rs"]
mod fixture;
#[path = "workflow/identity.rs"]
mod identity;
#[path = "workflow/plan.rs"]
mod plan;

use fixture::seat;

const PAIR: &str = "[workflow.hash.guard]\n\
\"rust\" = [\"crates\", \"Cargo.toml\"]\n\
\"web\" = [\"apps\"]\n";

#[test]
fn absent() {
    let root = seat("absent");
    root.declared(PAIR);
    let (text, ok) = root.verb("hash", "guard/gone", false);
    assert!(!ok, "{text}");
}
