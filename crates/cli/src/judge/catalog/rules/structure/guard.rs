use super::super::rule;

rule!(
    GUARD_CHECKS_RELEASE_PROFILE,
    "structure.guard-checks-release-profile",
    "Guard exercises the release profile",
    "A Rust repository guard type-checks the profile its consumers build, not \
     only the one its tests run.",
    "The guard wrapper source.",
    Mechanized,
    PLUMB,
    [ADOPTION, REPOSITORY]
);
rule!(
    GUARD_CONCURRENCY,
    "structure.guard-concurrency",
    "Guard cancels superseded runs",
    "The guard workflow shares the canonical concurrency group and cancels superseded work.",
    "The guard workflow concurrency block.",
    Mechanized,
    PLUMB,
    [REPOSITORY]
);
rule!(
    GUARD_RUNS_DOCTOR,
    "structure.guard-runs-doctor",
    "Guard runs plumb doctor",
    "The repository guard invokes the travelling shape check.",
    "The guard wrapper source.",
    Mechanized,
    PLUMB,
    [ADOPTION, REPOSITORY]
);
rule!(
    GUARD_RUNS_ECTROPY,
    "structure.guard-runs-ectropy",
    "Guard runs Ectropy explicitly",
    "The repository guard invokes Ectropy's errors-only checker directly.",
    "The guard wrapper source.",
    Mechanized,
    ECTROPY,
    [ADOPTION, ECTROPY_TAG, REPOSITORY]
);
rule!(
    GUARD_USES_CURRENT_ECTROPY_MODE,
    "structure.guard-uses-current-ectropy-mode",
    "Guard uses the current Ectropy mode",
    "The repository guard carries neither the retired strict mode nor debt mode.",
    "The guard wrapper Ectropy invocation.",
    Mechanized,
    ECTROPY,
    [ADOPTION, ECTROPY_TAG, REPOSITORY]
);
