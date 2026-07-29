use super::{Rule, rule};

rule!(
    CONFIG_VOCABULARY,
    "config.vocabulary-owned-by-product",
    "Products own config vocabulary",
    "The substrate exports config mechanism; each binary declares its own sections and values.",
    "Product config types and imports from the substrate.",
    Prose,
    PRODUCT,
    [CONFIGURATION, OWNERSHIP]
);
rule!(
    TEMPLATE_VARIABLES,
    "template.variables-owned-by-product",
    "Products own template variables",
    "The substrate owns template grammar while each caller owns the variables it supplies.",
    "Manifest documentation and the variable table passed to fill.",
    Prose,
    PRODUCT,
    [CONFIGURATION, OWNERSHIP]
);
rule!(
    TEMPLATE_ALIGNED,
    "template.manifest-variables-aligned",
    "Manifest variables match product vocabulary",
    "A manifest spells every injected environment key exactly as the target product derives it.",
    "Manifest templates compared with the target's cascade vocabulary.",
    Prose,
    PRODUCT,
    [CONFIGURATION]
);
rule!(
    HOME_CASCADE,
    "state.home-enters-through-cascade",
    "Data home enters through the cascade",
    "A stateful tool resolves one data home as ordinary four-layer configuration.",
    "Home resolution code and the product config surface.",
    Prose,
    PRODUCT,
    [CONFIGURATION, STATE]
);
rule!(
    RECORDS_UNDER_STATE,
    "state.records-under-state",
    "Machine records live under state/",
    "Machine-written records live below the tool's data-home state directory.",
    "The product's data-home layout.",
    Prose,
    PRODUCT,
    [STATE]
);
rule!(
    RECORDS_SCHEMA_VERSIONED,
    "state.records-schema-versioned",
    "State records carry schema versions",
    "Each machine-owned record names its shape version and unknown versions refuse.",
    "Serialized state shapes and their readers.",
    Prose,
    PRODUCT,
    [STATE]
);
rule!(
    RECORDS_WRITTEN_WHOLE,
    "state.records-written-whole",
    "State records are replaced atomically",
    "A state write lands beside the target and replaces it whole.",
    "Every durable state write path.",
    Prose,
    PRODUCT,
    [STATE]
);
rule!(
    RECORDS_NOT_CONFIG,
    "state.records-not-config",
    "State is not a config surface",
    "Machine state may be rewritten; human policy enters through the cascade.",
    "State and configuration documentation and code paths.",
    Prose,
    PRODUCT,
    [CONFIGURATION, STATE]
);
rule!(
    OWNERSHIP_TWO_SIDED,
    "state.ownership-two-sided",
    "Destructive ownership proof is two-sided",
    "Acting on a managed path requires both a registry record and an in-path marker naming the tool.",
    "The managed registry, target marker, and destructive operation.",
    Prose,
    PRODUCT,
    [OWNERSHIP, STATE]
);
rule!(
    METADATA_MONOTONIC,
    "release.metadata-monotonic",
    "Release metadata moves forward",
    "A release resolves prior published state and refuses regression or an accidental same-version rerun.",
    "Registry metadata and the computed release version.",
    Prose,
    RELEASE,
    [RELEASE_TAG]
);
rule!(
    GUARD_FRESH,
    "release.guard-fresh",
    "Release guard runs fresh",
    "A release runs the full guard without inheriting stale incremental artifacts.",
    "The release lane's guard invocation and cache posture.",
    Prose,
    RELEASE,
    [RELEASE_TAG]
);
rule!(
    MANIFEST_STAMPED,
    "release.manifest-stamped",
    "The published manifest is stamped",
    "A release stamps the manifest the publisher actually reads.",
    "The release stamp target and packaged manifest.",
    Prose,
    RELEASE,
    [CARGO_TAG, RELEASE_TAG]
);
rule!(
    DRY_RUN_ASSERTED,
    "release.dry-run-asserted",
    "Dry run proves the intended version",
    "Before publishing, the lane requires the publisher dry run to name the intended version.",
    "Dry-run output before the first irreversible action.",
    Prose,
    RELEASE,
    [RELEASE_TAG]
);
rule!(
    PUBLISH_IDEMPOTENT,
    "release.publish-idempotent",
    "Publishing is idempotent",
    "An already-present matching artifact is verified and skipped so repair reruns are safe.",
    "Registry presence and checksums before each publish.",
    Prose,
    RELEASE,
    [RELEASE_TAG]
);
rule!(
    REGISTRY_READBACK,
    "release.registry-readback",
    "Published artifacts are read back",
    "A lane verifies immutable artifacts from their registry rather than local state.",
    "Registry responses after publish.",
    Prose,
    RELEASE,
    [RELEASE_TAG]
);
rule!(
    SEAL_LAST,
    "release.seal-last",
    "Release identity is sealed last",
    "A lane records tags and moving metadata only after immutable artifacts verify.",
    "Ordering of publish, verification, metadata, and tags.",
    Prose,
    RELEASE,
    [RELEASE_TAG]
);
rule!(
    FAILURE_REPORTED,
    "release.failure-reported",
    "Release failures leave an operator trail",
    "A failed lane reports evidence somewhere operators can reach outside ephemeral CI logs.",
    "Failure handlers, issue or artifact output, and notification paths.",
    Prose,
    RELEASE,
    [RELEASE_TAG]
);
rule!(
    REHEARSAL_AVAILABLE,
    "release.rehearsal-available",
    "Release lanes offer rehearsal",
    "A release lane can exercise packaging without publishing, tagging, or touching credentials.",
    "Workflow inputs and the guarded irreversible steps.",
    Prose,
    RELEASE,
    [RELEASE_TAG]
);
rule!(
    LANE_REHEARSED,
    "release.lane-rehearsed",
    "Idle release lanes are rehearsed",
    "A lane idle across renames or restructuring is exercised before it is trusted.",
    "Recent lane execution and local packaging rehearsal.",
    Prose,
    RELEASE,
    [RELEASE_TAG]
);
rule!(
    SITE_DISPATCH_ONLY,
    "site.dispatch-only",
    "Site deployment is deliberate",
    "The deploy lane runs on explicit dispatch rather than every landing.",
    "The site deployment workflow triggers.",
    Prose,
    RELEASE,
    [RELEASE_TAG, SITE_TAG]
);
rule!(
    SITE_PURPOSE_KEY,
    "site.purpose-scoped-key",
    "Site credentials are purpose-scoped",
    "The deploy key carries only the account and zone permissions one site needs.",
    "Provider token policy and the workflow secret surface.",
    Prose,
    RELEASE,
    [OWNERSHIP, SITE_TAG]
);
rule!(
    SITE_ARTIFACT_IDENTITY,
    "site.artifact-stamps-identity",
    "Site artifacts declare identity",
    "The built health document carries the commit and version that produced it.",
    "Built health documents.",
    Prose,
    RELEASE,
    [SITE_TAG, WEB_TAG]
);
rule!(
    SITE_ROUTES_ARTIFACT,
    "site.routes-declared-by-artifact",
    "Site artifacts declare routes",
    "Published routes are discoverable from the built artifact.",
    "Built sitemap or prerendered documents.",
    Prose,
    RELEASE,
    [SITE_TAG, WEB_TAG]
);
rule!(
    SITE_SHIPPER_ARTIFACT,
    "site.shipper-reads-artifact",
    "Shippers inspect artifacts, not source",
    "The site shipper derives identity and routes from build output rather than application source.",
    "The ship wrapper's inputs and inspection paths.",
    Prose,
    RELEASE,
    [RELEASE_TAG, SITE_TAG]
);
rule!(
    SITE_STATES_SEPARATE,
    "site.states-separated",
    "Deploy, binding, and reachability stay separate",
    "A site ship reports upload, platform binding, and edge reachability as distinct states.",
    "The ship report and its three evidence sources.",
    Prose,
    RELEASE,
    [SITE_TAG]
);
rule!(
    SITE_BINDING_THREE_VALUED,
    "site.binding-three-valued",
    "Binding admits unknown",
    "A denied control-plane read is unknown rather than evidence that a domain is unbound.",
    "Provider binding lookup results and permission failures.",
    Prose,
    RELEASE,
    [SITE_TAG]
);
rule!(
    SITE_READBACK,
    "site.readback-proves-build",
    "Readback proves the served build",
    "Reachability compares a fingerprinted served asset with the built page.",
    "Built and edge-served asset fingerprints.",
    Prose,
    RELEASE,
    [SITE_TAG, WEB_TAG]
);
rule!(
    SKILL_STANDING,
    "skill.standing-accurate",
    "Skill standing matches its binary",
    "A released skill never claims enforcement its matching binary does not provide.",
    "The skill's standing references compared with the binary rule catalog.",
    Prose,
    SKILL,
    [SKILL_TAG]
);

#[rustfmt::skip]
pub fn all() -> Vec<&'static Rule> {
    vec![
        &CONFIG_VOCABULARY, &DRY_RUN_ASSERTED, &FAILURE_REPORTED, &GUARD_FRESH,
        &HOME_CASCADE, &LANE_REHEARSED, &MANIFEST_STAMPED, &METADATA_MONOTONIC,
        &OWNERSHIP_TWO_SIDED, &PUBLISH_IDEMPOTENT, &RECORDS_NOT_CONFIG,
        &RECORDS_SCHEMA_VERSIONED, &RECORDS_UNDER_STATE, &RECORDS_WRITTEN_WHOLE,
        &REGISTRY_READBACK, &REHEARSAL_AVAILABLE, &SEAL_LAST, &SITE_ARTIFACT_IDENTITY,
        &SITE_BINDING_THREE_VALUED, &SITE_DISPATCH_ONLY, &SITE_PURPOSE_KEY,
        &SITE_READBACK, &SITE_ROUTES_ARTIFACT, &SITE_SHIPPER_ARTIFACT,
        &SITE_STATES_SEPARATE, &SKILL_STANDING, &TEMPLATE_ALIGNED, &TEMPLATE_VARIABLES,
    ]
}
