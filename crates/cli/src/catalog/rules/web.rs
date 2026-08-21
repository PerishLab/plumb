use super::{Mechanism, rule};

rule!(DESIGN_DEPENDENCY, "web.design-dependency");
rule!(DESIGN_PLUGIN_DEPENDENCY, "web.design-plugin-dependency");
rule!(DESIGN_PLUGIN_ACTIVE, "web.design-plugin-active");
rule!(VITE_DOES_NOT_OWN_DISPATCH, "web.vite-does-not-own-dispatch");
rule!(VIEWS_MANIFEST_LOADED, "web.views-manifest-loaded");
rule!(VIEWS_MANIFEST_RENDERED, "web.views-manifest-rendered");
rule!(VIEWS_TYPES_DECLARED, "web.views-types-declared");
rule!(BUILD_SCRIPT_PRESENT, "web.build-script-present");
rule!(GUARD_BUILDS_WEB, "web.guard-builds-web");
rule!(VIEW_PATH_SEGMENT, "web.view-path-segment");
rule!(VIEW_FILE_KIND, "web.view-file-kind");
rule!(SVELTE_UNDER_COMPONENTS, "web.svelte-under-components");
rule!(CONVENTION_PATH_LOWERCASE, "web.convention-path-lowercase");
rule!(COMPONENT_FILE_KIND, "web.component-file-kind");
rule!(HOOK_FILE_KIND, "web.hook-file-kind");

pub fn mechanisms() -> Vec<&'static Mechanism> {
    vec![
        &BUILD_SCRIPT_PRESENT,
        &VIEWS_TYPES_DECLARED,
        &COMPONENT_FILE_KIND,
        &CONVENTION_PATH_LOWERCASE,
        &DESIGN_PLUGIN_ACTIVE,
        &DESIGN_PLUGIN_DEPENDENCY,
        &GUARD_BUILDS_WEB,
        &HOOK_FILE_KIND,
        &DESIGN_DEPENDENCY,
        &SVELTE_UNDER_COMPONENTS,
        &VIEW_FILE_KIND,
        &VIEW_PATH_SEGMENT,
        &VIEWS_MANIFEST_LOADED,
        &VIEWS_MANIFEST_RENDERED,
        &VITE_DOES_NOT_OWN_DISPATCH,
    ]
}
