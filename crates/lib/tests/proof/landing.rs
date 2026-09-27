use plumb::guard::Verified;
use plumb::landing::{Preparation, Ready, Refusal, Request};
use std::path::Path;

fn prepare(root: &Path, verified: Verified) -> Result<Preparation, Refusal> {
    Request {
        root,
        base: "main",
        title: "Linked delivery",
        body: "Refs PerishLab/probe#1",
    }
    .inspect()?
    .prepare(verified)
}

fn revalidate(root: &Path, expected: &Preparation, verified: Verified) -> Result<Ready, Refusal> {
    Request {
        root,
        base: "main",
        title: "Linked delivery",
        body: "Refs PerishLab/probe#1",
    }
    .revalidate(expected, verified)
}

#[test]
fn linked() {
    let prepare = prepare;
    let revalidate = revalidate;
    assert_eq!(plumb::landing::SCHEMA, "plumb.landing/v1");
    assert_eq!(std::mem::size_of_val(&prepare), 0);
    assert_eq!(std::mem::size_of_val(&revalidate), 0);
}
