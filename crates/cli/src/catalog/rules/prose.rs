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
    GENERATOR_STABLE,
    "release.generator-stable",
    "Release compilation uses current stable Plumb",
    "A permanent release lane resolves canonical stable Plumb once and records its version and template digest.",
    "Generator installation, capsule provenance, and retry boundaries.",
    Prose,
    RELEASE,
    [RELEASE_TAG]
);
rule!(
    EXACT_CREATE_ONLY,
    "release.exact-create-only",
    "Exact release identity is immutable",
    "Content objects are addressed by digest and an exact seal is created only when absent or already byte-identical.",
    "Object keys, exact seal conditionals, digests, and rerun behavior.",
    Prose,
    RELEASE,
    [RELEASE_TAG]
);
rule!(
    PUBLIC_READBACK,
    "release.public-readback",
    "Published objects are read back publicly",
    "A release fetches its exact seal and every named object through the public authority and verifies size and digest.",
    "Public responses compared with the exact seal.",
    Prose,
    RELEASE,
    [RELEASE_TAG]
);
rule!(
    CAPSULE_SEALED,
    "release.capsule-sealed",
    "A compiled capsule does not drift",
    "A release run compiles once and retries the same capsule rather than resolving a new generator or artifact set.",
    "Capsule creation boundary and retry inputs.",
    Prose,
    RELEASE,
    [RELEASE_TAG]
);
rule!(
    COORDINATOR_SINGLE,
    "release.coordinator-single",
    "One coordinator owns release state",
    "Platform jobs only produce product artifacts; one coordinator compiles, publishes, verifies, and activates.",
    "Workflow job graph and credential placement.",
    Prose,
    RELEASE,
    [RELEASE_TAG]
);
rule!(
    GENERATED_EPHEMERAL,
    "release.generated-ephemeral",
    "Generated delivery files stay out of source",
    "Managers, capsules, seals, and pointers are release outputs rather than checked-in product files.",
    "Repository paths compared with capsule output.",
    Prose,
    RELEASE,
    [OWNERSHIP, RELEASE_TAG]
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
    "The site CLI inputs and artifact inspection paths.",
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
        &CAPSULE_SEALED, &CONFIG_VOCABULARY, &COORDINATOR_SINGLE, &DRY_RUN_ASSERTED,
        &EXACT_CREATE_ONLY, &GENERATED_EPHEMERAL, &GENERATOR_STABLE, &GUARD_FRESH,
        &HOME_CASCADE, &MANIFEST_STAMPED, &OWNERSHIP_TWO_SIDED, &PUBLIC_READBACK,
        &RECORDS_NOT_CONFIG,
        &RECORDS_SCHEMA_VERSIONED, &RECORDS_UNDER_STATE, &RECORDS_WRITTEN_WHOLE,
        &SITE_ARTIFACT_IDENTITY, &SITE_BINDING_THREE_VALUED, &SITE_DISPATCH_ONLY,
        &SITE_PURPOSE_KEY, &SITE_READBACK, &SITE_ROUTES_ARTIFACT,
        &SITE_SHIPPER_ARTIFACT, &SITE_STATES_SEPARATE, &SKILL_STANDING,
        &TEMPLATE_ALIGNED, &TEMPLATE_VARIABLES,
    ]
}
