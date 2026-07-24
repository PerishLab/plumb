mod support;

use plumb::context::Context;
use plumb::trace::{Ledger, Sink, Span};
use std::sync::Arc;

#[test]
fn emits() {
    let ledger = Arc::new(Ledger::default());
    let context = Context::root().with(Sink::from(ledger.clone()));
    {
        let _scope = context.enter();
        let scoped = Context::current().with(Span::open("turn"));
        let _inner = scoped.enter();
    }
    let seen = ledger.query(|record| record.name == "turn");
    assert_eq!(seen.len(), 1);
    assert!(seen[0].parent.is_none());
    assert!(seen[0].closed >= seen[0].opened);
}

#[test]
fn silent() {
    let scoped = Context::root().with(Span::open("orphan"));
    let _scope = scoped.enter();
    Span::note("seen", "true");
}

#[test]
fn lineage() {
    let ledger = Arc::new(Ledger::default());
    let context = Context::root().with(Sink::from(ledger.clone()));
    {
        let _scope = context.enter();
        let outer = Context::current().with(Span::open("turn"));
        {
            let _outer = outer.enter();
            let inner = Context::current().with(Span::open("effect.dispatch"));
            let _inner = inner.enter();
        }
    }
    let turns = ledger.query(|record| record.name == "turn");
    let effects = ledger.query(|record| record.name == "effect.dispatch");
    assert_eq!(effects[0].parent, Some(turns[0].id));
}

#[test]
fn notes() {
    let ledger = Arc::new(Ledger::default());
    let context = Context::root().with(Sink::from(ledger.clone()));
    {
        let _scope = context.enter();
        let scoped = Context::current().with(Span::open("effect.dispatch"));
        let _inner = scoped.enter();
        Span::note("effect.reason", "operator_resolved");
    }
    let seen = ledger.query(|record| {
        record.name == "effect.dispatch" && record.says("effect.reason", "operator_resolved")
    });
    assert_eq!(seen.len(), 1);
}

#[test]
fn queries() {
    let ledger = Arc::new(Ledger::default());
    let context = Context::root().with(Sink::from(ledger.clone()));
    {
        let _scope = context.enter();
        for reason in ["advanced", "operator_resolved", "advanced"] {
            let scoped = Context::current().with(Span::open("effect.dispatch"));
            let _inner = scoped.enter();
            Span::note("effect.reason", reason);
        }
    }
    let advanced = ledger.query(|record| record.says("effect.reason", "advanced"));
    let resolved = ledger.query(|record| record.says("effect.reason", "operator_resolved"));
    assert_eq!(advanced.len(), 2);
    assert_eq!(resolved.len(), 1);
}

#[test]
fn carries() {
    let ledger = Arc::new(Ledger::default());
    let context = Context::root().with(Sink::from(ledger.clone()));
    support::block(context.carry(async {
        let scoped = Context::current().with(Span::open("turn"));
        scoped
            .carry(async {
                Span::note("phase", "worked");
                support::rest().await;
            })
            .await;
    }));
    let seen = ledger.query(|record| record.name == "turn" && record.says("phase", "worked"));
    assert_eq!(seen.len(), 1);
}
