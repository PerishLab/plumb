use super::{Mechanism, rule};

rule!(SPEC_DECLARED, "release.spec-declared");

rule!(ATTACHMENT_DELIVERABLE, "release.attachment-deliverable");

rule!(LANE_RENDERED, "release.lane-rendered");

rule!(DATUM_RECORDED, "release.datum-recorded");

rule!(ATTACHMENT_PERMITTED, "release.attachment-permitted");

rule!(ATTACHMENT_EXERCISED, "release.attachment-exercised");

pub fn mechanisms() -> Vec<&'static Mechanism> {
    vec![
        &ATTACHMENT_DELIVERABLE,
        &ATTACHMENT_EXERCISED,
        &ATTACHMENT_PERMITTED,
        &DATUM_RECORDED,
        &LANE_RENDERED,
        &SPEC_DECLARED,
    ]
}
