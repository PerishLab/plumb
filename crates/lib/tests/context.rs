mod support;

use plumb::context::Context;

struct Who(&'static str);
struct Limit(u32);

#[test]
fn derives() {
    let root = Context::root();
    let derived = root.with(Who("perish"));
    assert!(root.get::<Who>().is_none());
    assert_eq!(derived.get::<Who>().unwrap().0, "perish");
}

#[test]
fn shadows() {
    let context = Context::root().with(Limit(1)).with(Limit(2));
    assert_eq!(context.get::<Limit>().unwrap().0, 2);
}

#[test]
fn layers() {
    let context = Context::root().with(Who("perish")).with(Limit(9));
    assert_eq!(context.get::<Who>().unwrap().0, "perish");
    assert_eq!(context.get::<Limit>().unwrap().0, 9);
}

#[test]
fn ambient() {
    assert!(Context::current().get::<Who>().is_none());
    let context = Context::root().with(Who("perish"));
    {
        let _scope = context.enter();
        assert_eq!(Context::current().get::<Who>().unwrap().0, "perish");
    }
    assert!(Context::current().get::<Who>().is_none());
}

#[test]
fn nests() {
    let outer = Context::root().with(Who("outer"));
    let _scope = outer.enter();
    {
        let inner = Context::current().with(Who("inner"));
        let _scope = inner.enter();
        assert_eq!(Context::current().get::<Who>().unwrap().0, "inner");
    }
    assert_eq!(Context::current().get::<Who>().unwrap().0, "outer");
}

#[test]
fn carries() {
    let context = Context::root().with(Who("perish"));
    let seen = support::block(context.carry(async {
        let before = Context::current().get::<Who>().unwrap().0;
        support::rest().await;
        let after = Context::current().get::<Who>().unwrap().0;
        (before, after)
    }));
    assert_eq!(seen, ("perish", "perish"));
    assert!(Context::current().get::<Who>().is_none());
}

#[test]
fn travels() {
    let context = Context::root().with(Who("perish"));
    let carried = context.carry(async { Context::current().get::<Who>().unwrap().0 });
    let seen = std::thread::spawn(move || support::block(carried))
        .join()
        .unwrap();
    assert_eq!(seen, "perish");
}
