use super::{Rule, rule};

rule!(
    PATHS_VALID,
    "lock.paths-valid",
    "Lock paths exist and are readable",
    "Every path named by a cross-boundary affirmation exists and can be read.",
    "The paths declared by each plumb.toml lock.",
    Mechanized,
    PLUMB,
    [OWNERSHIP, REPOSITORY]
);
rule!(
    CONTENT_CURRENT,
    "lock.content-current",
    "Locked content matches its affirmation",
    "The bytes covered by a lock match the hash recorded at the last human reading.",
    "Normalized paths and bytes covered by each lock.",
    Mechanized,
    PLUMB,
    [OWNERSHIP, REPOSITORY]
);
rule!(
    VERSION_CURRENT,
    "lock.version-current",
    "Locks are reread before a version cut",
    "A repository version does not move past the version at which a lock was affirmed.",
    "The repository version and each lock's recorded version.",
    Mechanized,
    PLUMB,
    [OWNERSHIP, RELEASE_TAG]
);

pub fn all() -> Vec<&'static Rule> {
    vec![&CONTENT_CURRENT, &PATHS_VALID, &VERSION_CURRENT]
}
