use super::finding::{Found, wrong};
use crate::catalog::model::Mechanism;
use crate::catalog::rules::web as rule;
use crate::shape::web::{Evidence, Role};

pub fn judge(evidence: Option<&Evidence>) -> Found {
    let Some(evidence) = evidence else {
        return Found::new();
    };
    let mut found = Found::new();
    let plane = &evidence.plane;
    check(
        &mut found,
        plane.design,
        &rule::DESIGN_DEPENDENCY,
        "web does not depend on @perish/design",
    );
    check(
        &mut found,
        plane.plugin,
        &rule::DESIGN_PLUGIN_ACTIVE,
        "vite does not activate the design plugin",
    );
    check(
        &mut found,
        plane.dispatch,
        &rule::VITE_DOES_NOT_OWN_DISPATCH,
        "vite manually consumes sidecar dispatch environment",
    );
    check(
        &mut found,
        plane.loaded,
        &rule::VIEWS_MANIFEST_LOADED,
        "web does not load the virtual views manifest",
    );
    check(
        &mut found,
        plane.rendered,
        &rule::VIEWS_MANIFEST_RENDERED,
        "web does not render the views manifest",
    );
    check(
        &mut found,
        plane.typed,
        &rule::VIEWS_TYPES_DECLARED,
        "web does not declare the virtual views module type",
    );
    check(
        &mut found,
        plane.build,
        &rule::BUILD_SCRIPT_PRESENT,
        "web has no build script",
    );
    check(
        &mut found,
        plane.guarded,
        &rule::GUARD_BUILDS_WEB,
        "guard does not build the web app",
    );
    for role in &evidence.roles {
        let (rule, evidence) = match role {
            Role::Segment => (
                &rule::VIEW_PATH_SEGMENT,
                "web view paths must be lowercase route segments",
            ),
            Role::View => (
                &rule::VIEW_FILE_KIND,
                "web views only hold route svelte files",
            ),
            Role::Direct => (
                &rule::SVELTE_UNDER_COMPONENTS,
                "web lib svelte must live under lib/components",
            ),
            Role::Case => (
                &rule::CONVENTION_PATH_LOWERCASE,
                "web convention paths must be lowercase",
            ),
            Role::Component => (
                &rule::COMPONENT_FILE_KIND,
                "web components only hold lowercase svelte files",
            ),
            Role::Hook => (
                &rule::HOOK_FILE_KIND,
                "web hooks must be lowercase .ts files",
            ),
        };
        found.push(wrong(rule, evidence));
    }
    found
}

fn check(found: &mut Found, valid: bool, rule: &'static Mechanism, evidence: &'static str) {
    if !valid {
        found.push(wrong(rule, evidence));
    }
}
