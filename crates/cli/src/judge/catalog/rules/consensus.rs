use super::{Rule, rule};

rule!(
    CANONICAL_AUTHORITY,
    "release.canonical-authority",
    "Stable identity includes its authority",
    "A stable label admits a default seat only when its release authority is the product's canonical authority.",
    "Manager source selection, canonical release URL, and the resolved install paths.",
    Prose,
    RELEASE,
    [OWNERSHIP, RELEASE_TAG]
);
rule!(
    DEFAULT_SEAT_STABLE,
    "release.default-seat-stable",
    "Stable alone occupies default seats",
    "Canonical install roots and command entrypoints admit stable releases and refuse every non-stable channel.",
    "Manager channel, effective install root, effective bin directory, and refusal ordering.",
    Prose,
    RELEASE,
    [ADOPTION, OWNERSHIP, RELEASE_TAG]
);
rule!(
    CANONICAL_MANAGER_STABLE,
    "release.canonical-manager-stable",
    "Canonical managers belong to stable",
    "Root public managers change only during stable activation; every exact seal records generated content-addressed managers.",
    "Release object keys, manager digests, stable activation ordering, and public readback.",
    Prose,
    RELEASE,
    [OWNERSHIP, RELEASE_TAG]
);
rule!(
    NONSTABLE_EXACT,
    "release.nonstable-exact",
    "Non-stable install intent is exact",
    "Every non-stable consumer names both its channel and one exact sealed version; no non-stable pointer exists.",
    "Manager and workflow inputs compared with the exact release seal.",
    Prose,
    RELEASE,
    [ADOPTION, RELEASE_TAG]
);
rule!(
    NONSTABLE_ISOLATED,
    "release.nonstable-isolated",
    "Non-stable installs are task-isolated",
    "A non-stable release requires explicit install and command paths provably disjoint from stable default seats.",
    "Effective normalized paths, symlink resolution, channel, and writes performed after the refusal gate.",
    Prose,
    RELEASE,
    [ADOPTION, OWNERSHIP, RELEASE_TAG]
);
rule!(
    STABLE_SINGLE_WRITER,
    "release.stable-single-writer",
    "Stable default seats have one writer",
    "Default-seat mutation is locked, ownership-proven, staged, atomically switched, and monotonic unless an exact rollback is explicit.",
    "Manager locks, markers, staging paths, entrypoint replacement, and installed-versus-selected versions.",
    Prose,
    RELEASE,
    [OWNERSHIP, RELEASE_TAG, STATE]
);
rule!(
    PROMOTION_PROOF_EXACT,
    "release.promotion-proof-exact",
    "Stable records an exact candidate proof",
    "Stable promotion embeds one complete non-stable exact seal and its digest, with the same product, base version, and source commit.",
    "Stable seal, exact candidate seal and digest, source commits, and pre-publish revalidation.",
    Prose,
    RELEASE,
    [OWNERSHIP, RELEASE_TAG]
);
rule!(
    STABLE_SOURCE_LINE,
    "release.stable-source-line",
    "Stable alone binds a release line",
    "Exact publication may bind any selected branch ref, while stable binds only refs/heads/release/vX.Y.Z and freezes that ref and commit before publication.",
    "Dispatch event ref and commit, release channel and version, checkout HEAD, and stable branch protection state.",
    Prose,
    RELEASE,
    [OWNERSHIP, RELEASE_TAG, STATE]
);
rule!(
    CANONICAL_BRANCH_PROTECTION,
    "structure.canonical-branch-protection",
    "Managed repositories share branch protection",
    "Every Plumb-managed repository carries byte-identical main and release/** baseline rules; exact release rules follow only PREPARING or FROZEN state, and settled release branches remain frozen.",
    "Forgejo branch-protection responses projected onto the canonical skill documents, existing release branches, and stable consensus.",
    Prose,
    PLUMB,
    [RELEASE_TAG, REPOSITORY, STATE]
);
rule!(
    STABLE_PACKPORT,
    "release.stable-packport",
    "Stable settles back into main",
    "After stable activation a topology-preserving merge makes its published commit an ancestor of main before another stable activation; exact release and ordinary main movement remain independent.",
    "Stable pointer commit, main ancestry, release-line merge topology, and the next stable activation gate.",
    Prose,
    RELEASE,
    [OWNERSHIP, RELEASE_TAG, STATE]
);
rule!(
    SKILL_MANAGED_STABLE,
    "skill.managed-stable-only",
    "Managed skill seats admit stable",
    "Install, status, and upgrade keep the global skill ledger anchored to stable releases.",
    "Managed skill command inputs, stable pointer, exact seal, and ledger writes.",
    Prose,
    SKILL,
    [ADOPTION, OWNERSHIP, SKILL_TAG, STATE]
);
rule!(
    SKILL_CANDIDATE_STAGE,
    "skill.candidate-stage-isolated",
    "Candidate skills stage outside the ledger",
    "A non-stable skill requires an exact version and a new explicit path, receives a staged marker, and never enters managed state.",
    "Stage inputs, target absence, staged marker, and unchanged managed ledger.",
    Prose,
    SKILL,
    [ADOPTION, OWNERSHIP, SKILL_TAG, STATE]
);

#[rustfmt::skip]
pub fn all() -> Vec<&'static Rule> {
    vec![
        &CANONICAL_AUTHORITY, &CANONICAL_BRANCH_PROTECTION, &CANONICAL_MANAGER_STABLE,
        &DEFAULT_SEAT_STABLE, &NONSTABLE_EXACT, &NONSTABLE_ISOLATED,
        &PROMOTION_PROOF_EXACT, &SKILL_CANDIDATE_STAGE, &SKILL_MANAGED_STABLE,
        &STABLE_PACKPORT, &STABLE_SINGLE_WRITER, &STABLE_SOURCE_LINE,
    ]
}
