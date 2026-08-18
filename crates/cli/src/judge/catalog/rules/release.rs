use super::{Rule, rule};

rule!(
    SPEC_DECLARED,
    "release.spec-declared",
    "Products declare release inputs once",
    "One strict product spec owns authority, version probe, binaries, platform archives, and extra assets, and the current Plumb reads it whole or refuses it by name.",
    "The release table in plumb.toml read by the same strict spec reader every release verb uses.",
    Mechanized,
    RELEASE,
    [OWNERSHIP, RELEASE_TAG]
);

rule!(
    ATTACHMENT_DELIVERABLE,
    "release.attachment-deliverable",
    "Declared attachments resolve to a delivering lane",
    "Every declared release attachment resolves to a shared release lane that carries its projection and that this repository actually calls.",
    "The release attachment tables in plumb.toml and the shared workflows named by the repository's release callers.",
    Mechanized,
    RELEASE,
    [RELEASE_TAG, REPOSITORY]
);

rule!(
    LANE_RENDERED,
    "release.lane-rendered",
    "Release lanes stand as this Plumb renders them",
    "Every governed lane in a repository is the exact text the current Plumb renders from that repository's declaration, so no lane is hand-written or left behind by an older Plumb.",
    "The lane templates carried by the running Plumb, the release table in plumb.toml, and the tracked workflow bytes.",
    Mechanized,
    RELEASE,
    [RELEASE_TAG, REPOSITORY]
);

rule!(
    DATUM_RECORDED,
    "release.datum-recorded",
    "Release lines judge against a recorded datum",
    "A release line carries the stable answers its judgement rests on, recorded once when the line is cut, so every later judgement measures the frozen candidate against that datum instead of a registry that moves under it.",
    "The datum leaf under the mechanism seat in the release line's own tree.",
    Mechanized,
    RELEASE,
    [DEPENDENCY, RELEASE_TAG]
);

rule!(
    ATTACHMENT_PERMITTED,
    "release.attachment-permitted",
    "Attachments declare no more than Plumb permits",
    "An attachment declares at most the number of packages Plumb permits it, because every package past the first adds ordering, cross-pinning, and partial failure to the mechanism, and a product cannot grant itself a wider shape by declaring one.",
    "The attachment tables in plumb.toml against the permitted widths in Plumb's release rules.",
    Mechanized,
    RELEASE,
    [OWNERSHIP, RELEASE_TAG]
);

rule!(
    ATTACHMENT_EXERCISED,
    "release.attachment-exercised",
    "Unexercised width stays visible",
    "The width a product declares is reported against the width this skeleton has actually released, so a path no release has ever run says so instead of passing as proven.",
    "The attachment tables in plumb.toml against the exercised widths in Plumb's release rules.",
    Mechanized,
    RELEASE,
    [RELEASE_TAG, REPOSITORY]
);

pub fn all() -> Vec<&'static Rule> {
    vec![
        &ATTACHMENT_DELIVERABLE,
        &ATTACHMENT_EXERCISED,
        &ATTACHMENT_PERMITTED,
        &DATUM_RECORDED,
        &LANE_RENDERED,
        &SPEC_DECLARED,
    ]
}
