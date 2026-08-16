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

pub fn all() -> Vec<&'static Rule> {
    vec![&ATTACHMENT_DELIVERABLE, &SPEC_DECLARED]
}
