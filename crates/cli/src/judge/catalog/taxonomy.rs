use serde::Serialize;

pub struct Owner {
    pub id: &'static str,
    pub summary: &'static str,
}

pub struct Tag {
    pub id: &'static str,
    pub summary: &'static str,
}

pub struct Namespace {
    pub id: &'static str,
    pub summary: &'static str,
    pub owner: &'static Owner,
}

macro_rules! item {
    ($name:ident, $kind:ident, $id:literal, $summary:literal) => {
        pub static $name: $kind = $kind {
            id: $id,
            summary: $summary,
        };
    };
}

item!(PLUMB, Owner, "plumb", "the travelling repository skeleton");
item!(ECTROPY, Owner, "ectropy", "the syntax and policy checker");
item!(
    CARGO,
    Owner,
    "cargo",
    "the Rust package and release boundary"
);
item!(WEB, Owner, "web", "the web application layer");
item!(
    SIDECAR,
    Owner,
    "sidecar",
    "the local process dispatch layer"
);
item!(
    RELEASE,
    Owner,
    "release",
    "the publishing and deployment lane"
);
item!(
    PRODUCT,
    Owner,
    "product",
    "the binary that owns its vocabulary"
);
item!(SKILL, Owner, "skill", "the released agent operating brief");

pub static OWNERS: &[&Owner] = &[
    &CARGO, &ECTROPY, &PLUMB, &PRODUCT, &RELEASE, &SIDECAR, &SKILL, &WEB,
];

item!(ADOPTION, Tag, "adoption", "operator entrypoint adoption");
item!(CARGO_TAG, Tag, "cargo", "Rust workspace and package shape");
item!(
    CONFIGURATION,
    Tag,
    "configuration",
    "runtime configuration and templates"
);
item!(
    DEPENDENCY,
    Tag,
    "dependency",
    "dependency naming and policy"
);
item!(DISPATCH, Tag, "dispatch", "process dispatch and readiness");
item!(ECTROPY_TAG, Tag, "ectropy", "ectropy policy and execution");
item!(OWNERSHIP, Tag, "ownership", "resource ownership boundaries");
item!(RELEASE_TAG, Tag, "release", "release and deployment lanes");
item!(REPOSITORY, Tag, "repository", "repository skeleton shape");
item!(SITE_TAG, Tag, "site", "site build and deployment shape");
item!(SKILL_TAG, Tag, "skill", "agent operating briefs");
item!(STATE, Tag, "state", "machine-owned durable state");
item!(WEB_TAG, Tag, "web", "web application and site shape");

pub static TAGS: &[&Tag] = &[
    &ADOPTION,
    &CARGO_TAG,
    &CONFIGURATION,
    &DEPENDENCY,
    &DISPATCH,
    &ECTROPY_TAG,
    &OWNERSHIP,
    &RELEASE_TAG,
    &REPOSITORY,
    &SITE_TAG,
    &SKILL_TAG,
    &STATE,
    &WEB_TAG,
];

macro_rules! namespace {
    ($name:ident, $id:literal, $summary:literal, $owner:ident) => {
        pub static $name: Namespace = Namespace {
            id: $id,
            summary: $summary,
            owner: &$owner,
        };
    };
}

namespace!(CONFIG, "config", "runtime policy vocabulary", PRODUCT);
namespace!(DEPS, "deps", "dependency posture", PLUMB);
namespace!(DISPATCH_NS, "dispatch", "process dispatch shape", SIDECAR);
namespace!(ENV, "env", "repository execution environment", PLUMB);
namespace!(LOCK, "lock", "cross-boundary affirmations", PLUMB);
namespace!(RELEASE_NS, "release", "release lane anatomy", RELEASE);
namespace!(SKILL_NS, "skill", "released agent brief", SKILL);
namespace!(STATE_NS, "state", "machine-owned state", PRODUCT);
namespace!(STRUCTURE, "structure", "repository skeleton", PLUMB);
namespace!(TEMPLATE, "template", "config text variables", PRODUCT);
namespace!(SITE, "site", "site deployment semantics", RELEASE);
namespace!(WEB_NS, "web", "web application shape", WEB);

pub static NAMESPACES: &[&Namespace] = &[
    &CONFIG,
    &DEPS,
    &DISPATCH_NS,
    &ENV,
    &LOCK,
    &RELEASE_NS,
    &SITE,
    &SKILL_NS,
    &STATE_NS,
    &STRUCTURE,
    &TEMPLATE,
    &WEB_NS,
];

#[derive(Serialize)]
pub struct OwnerView {
    pub id: &'static str,
    pub summary: &'static str,
}

#[derive(Serialize)]
pub struct TagView {
    pub id: &'static str,
    pub summary: &'static str,
}

#[derive(Serialize)]
pub struct NamespaceView {
    pub id: &'static str,
    pub summary: &'static str,
    pub owner: &'static str,
}

impl From<&'static Owner> for OwnerView {
    fn from(owner: &'static Owner) -> Self {
        Self {
            id: owner.id,
            summary: owner.summary,
        }
    }
}

impl From<&'static Tag> for TagView {
    fn from(tag: &'static Tag) -> Self {
        Self {
            id: tag.id,
            summary: tag.summary,
        }
    }
}

impl From<&'static Namespace> for NamespaceView {
    fn from(namespace: &'static Namespace) -> Self {
        Self {
            id: namespace.id,
            summary: namespace.summary,
            owner: namespace.owner.id,
        }
    }
}
