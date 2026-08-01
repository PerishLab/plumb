use super::support;

use plumb::context::Context;
use plumb::trace::{Ledger, Sink, Span, span};
use std::sync::Arc;

#[span("work.step")]
fn step(count: u32) -> u32 {
    Span::note("count", &count.to_string());
    count + 1
}

#[span("work.flow")]
async fn flow() -> u32 {
    support::rest().await;
    step(1)
}

#[test]
fn wraps() {
    let ledger = Arc::new(Ledger::default());
    let context = Context::root().with(Sink::from(ledger.clone()));
    {
        let _scope = context.enter();
        assert_eq!(step(1), 2);
    }
    let seen = ledger.query(|record| record.name == "work.step" && record.says("count", "1"));
    assert_eq!(seen.len(), 1);
}

#[test]
fn weaves() {
    let ledger = Arc::new(Ledger::default());
    let context = Context::root().with(Sink::from(ledger.clone()));
    assert_eq!(support::block(context.carry(flow())), 2);
    let flows = ledger.query(|record| record.name == "work.flow");
    let steps = ledger.query(|record| record.name == "work.step");
    assert_eq!(flows.len(), 1);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].parent, Some(flows[0].id));
}
