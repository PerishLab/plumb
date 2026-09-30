use super::{Line, released, unreleased, value};
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
fn ascent() {
    let judged = |marker| value::ascends(LISTING, "origin", marker);
    judged("v1.2.0").expect("a stable above its own prerelease");
    judged("v1.2.0-rc.2").expect("the next prerelease");
    judged("v1.2.0-rc.1").expect("the highest marker itself");
    for lower in ["v1.1.0", "v1.1.1-rc.1", "v1.2.0-beta.1", "v1.0.0"] {
        let refused = judged(lower).expect_err(lower);
        assert!(
            refused.contains("does not exceed v1.2.0-rc.1, the highest marker origin holds"),
            "{refused}"
        );
        assert!(refused.contains("[release.marker-ordered]"), "{refused}");
    }
    let stray = format!("{LISTING}x\trefs/tags/nightly\ny\trefs/tags/v9\n");
    value::ascends(&stray, "origin", "v1.2.0").expect("names that are no version do not count");
}

#[test]
fn abandoned() {
    let held = line();
    unreleased(&held, "v1.2.0", "r2", "release/v1.2.0").expect("a line resting on its rc closes");
    let refused = unreleased(&held, "v1.1.0", "l", "release/v1.1.0").expect_err("unrecorded head");
    assert!(
        refused.contains("neither main nor any marker of v1.1.0 holds"),
        "{refused}"
    );
}

#[test]
fn moved() {
    let refused = released(&line(), "v1.0.0", "c1", "later").expect_err("past stable");
    assert!(refused.contains("past stable v1.0.0"), "{refused}");
}
