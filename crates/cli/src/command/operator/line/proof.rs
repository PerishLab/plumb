use super::{Line, released, unreleased};
use crate::shape::release::Spec;
use std::path::PathBuf;

const LISTING: &str = "m\trefs/heads/main\nl\trefs/heads/release/v1.1.0\nt1\trefs/tags/v1.0.0\nc1\trefs/tags/v1.0.0^{}\nt2\trefs/tags/v1.1.0-rc.1\nr1\trefs/tags/v1.1.0-rc.1^{}\nt3\trefs/tags/v1.2.0-rc.1\nr2\trefs/tags/v1.2.0-rc.1^{}\n";

fn line() -> Line {
    let root = PathBuf::from(".");
    let spec = Spec::decode(
        &root,
        "[release]\nproduct = \"demo\"\nauthority = \"https://releases.demo.example\"\nbinaries = [\"demo\"]\ntargets = [\"x86_64-unknown-linux-gnu\"]\n",
        "fixture",
    )
    .expect("spec");
    Line {
        root,
        remote: "origin".into(),
        listing: LISTING.into(),
        spec,
    }
}

#[test]
fn origins() {
    let held = line();
    assert_eq!(held.origin("main", "v1.1.0").as_deref(), Ok("m"));
    assert_eq!(held.origin("v1.0.0", "v1.0.1").as_deref(), Ok("c1"));
    assert!(
        held.origin("v1.0.0", "v1.0.0")
            .expect_err("same")
            .contains("opens from below")
    );
    assert!(
        held.origin("v1.1.0-rc.1", "v1.2.0")
            .expect_err("prerelease")
            .contains("stable marker")
    );
    assert!(
        held.origin("v0.9.0", "v1.0.0")
            .expect_err("absent")
            .contains("holds no stable marker")
    );
}

#[test]
fn abandoned() {
    let held = line();
    unreleased(&held, "v1.2.0", "r2", "release/v1.2.0").expect("a line resting on its rc closes");
    let refused = unreleased(&held, "v1.1.0", "l", "release/v1.1.0").expect_err("unrecorded head");
    assert!(refused.contains("no marker of v1.1.0 records"), "{refused}");
}

#[test]
fn moved() {
    let refused = released(&line(), "v1.0.0", "c1", "later").expect_err("past stable");
    assert!(refused.contains("past stable v1.0.0"), "{refused}");
}
