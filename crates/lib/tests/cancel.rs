mod support;

use plumb::cancel::Cancel;
use plumb::context::Context;
use std::time::Duration;

#[test]
fn fires() {
    let cancel = Cancel::root();
    assert!(!cancel.cancelled());
    cancel.cancel();
    assert!(cancel.cancelled());
}

#[test]
fn descends() {
    let parent = Cancel::root();
    let child = parent.fork();
    parent.cancel();
    assert!(child.cancelled());
}

#[test]
fn contains() {
    let parent = Cancel::root();
    let child = parent.fork();
    child.cancel();
    assert!(child.cancelled());
    assert!(!parent.cancelled());
}

#[test]
fn ambient() {
    let (outer, context) = Cancel::child();
    let _scope = context.enter();
    let (inner, _derived) = Cancel::child();
    outer.cancel();
    assert!(inner.cancelled());
}

#[test]
fn settled() {
    let cancel = Cancel::root();
    cancel.cancel();
    support::block(cancel.done());
}

#[test]
fn waits() {
    let cancel = Cancel::root();
    let held = cancel.clone();
    let waiter = std::thread::spawn(move || support::block(held.done()));
    std::thread::sleep(Duration::from_millis(20));
    cancel.cancel();
    waiter.join().unwrap();
}

#[test]
fn wakes() {
    let parent = Cancel::root();
    let child = parent.fork();
    let waiter = std::thread::spawn(move || support::block(child.done()));
    std::thread::sleep(Duration::from_millis(20));
    parent.cancel();
    waiter.join().unwrap();
}

#[test]
fn cooperates() {
    let (cancel, context) = Cancel::child();
    let worker = std::thread::spawn(move || {
        support::block(context.carry(async {
            let mut rounds = 0u32;
            loop {
                let held = Context::current().get::<Cancel>().unwrap();
                if held.cancelled() {
                    return rounds;
                }
                rounds += 1;
                support::rest().await;
            }
        }))
    });
    std::thread::sleep(Duration::from_millis(20));
    cancel.cancel();
    assert!(worker.join().unwrap() > 0);
}
