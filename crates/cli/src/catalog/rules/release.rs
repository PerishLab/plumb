use super::{Mechanism, rule};

rule!(SPEC_DECLARED, "release.spec-declared");

rule!(DATUM_RECORDED, "release.datum-recorded");

rule!(ATTACHMENT_PERMITTED, "release.attachment-permitted");

rule!(ATTACHMENT_EXERCISED, "release.attachment-exercised");

rule!(STABLE_REJOIN, "release.stable-rejoin");

pub fn mechanisms() -> Vec<&'static Mechanism> {
    vec![
        &ATTACHMENT_EXERCISED,
        &ATTACHMENT_PERMITTED,
        &DATUM_RECORDED,
        &SPEC_DECLARED,
        &STABLE_REJOIN,
    ]
}
