use super::{Mechanism, rule};

rule!(SPEC_DECLARED, "release.spec-declared");

rule!(DATUM_RECORDED, "release.datum-recorded");

rule!(ATTACHMENT_PERMITTED, "release.attachment-permitted");

rule!(ATTACHMENT_EXERCISED, "release.attachment-exercised");

rule!(STABLE_REJOIN, "release.stable-rejoin");

rule!(STABLE_LODGED, "release.stable-lodged");

rule!(LINE_CLOSED, "release.line-closed");

pub fn mechanisms() -> Vec<&'static Mechanism> {
    vec![
        &ATTACHMENT_EXERCISED,
        &ATTACHMENT_PERMITTED,
        &DATUM_RECORDED,
        &LINE_CLOSED,
        &SPEC_DECLARED,
        &STABLE_LODGED,
        &STABLE_REJOIN,
    ]
}
